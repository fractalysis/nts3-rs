use core::marker::PhantomData;
use core::mem::size_of;
use core::ptr;

const CHANNELS: usize = 2;

/// Error returned when SDK audio pointers cannot form a stereo render buffer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BufferError {
    NullPointer,
    SizeOverflow,
    PartialOverlap,
}

/// One render call's interleaved stereo input and output buffers.
///
/// The type intentionally exposes copied samples rather than slices or sample
/// references. This keeps exact in-place input/output valid without ever
/// creating aliased Rust references.
pub struct StereoBuffer<'audio> {
    input: *const f32,
    output: *mut f32,
    raw_input: Option<*const f32>,
    frames: usize,
    _lifetime: PhantomData<&'audio mut f32>,
}

impl<'audio> StereoBuffer<'audio> {
    /// Builds a buffer over one SDK render call.
    ///
    /// # Safety
    /// For nonzero `frames`, `input` must be readable and `output` writable for
    /// `frames * 2` initialized `f32` values. They must be disjoint or exactly
    /// equal. A non-null `raw_input` must be readable for the same length. None
    /// of the pointers may be used outside `'audio`.
    pub(crate) unsafe fn from_raw(
        input: *const f32,
        output: *mut f32,
        raw_input: *const f32,
        frames: usize,
    ) -> Result<Self, BufferError> {
        let samples = frames
            .checked_mul(CHANNELS)
            .ok_or(BufferError::SizeOverflow)?;
        let bytes = samples
            .checked_mul(size_of::<f32>())
            .ok_or(BufferError::SizeOverflow)?;

        if frames != 0 && (input.is_null() || output.is_null()) {
            return Err(BufferError::NullPointer);
        }

        if cfg!(debug_assertions)
            && bytes != 0
            && input.cast_mut() != output
            && ranges_overlap(input as usize, output as usize, bytes)?
        {
            return Err(BufferError::PartialOverlap);
        }

        Ok(Self {
            input,
            output,
            raw_input: (!raw_input.is_null()).then_some(raw_input),
            frames,
            _lifetime: PhantomData,
        })
    }

    pub const fn len(&self) -> usize {
        self.frames
    }

    pub const fn is_empty(&self) -> bool {
        self.frames == 0
    }

    /// Returns the raw, pre-routing stereo input for this render call, when the
    /// runtime provides it. Samples are copied out and no pointer can escape.
    pub const fn raw_input(&self) -> Option<StereoInput<'audio>> {
        match self.raw_input {
            Some(pointer) => Some(StereoInput {
                pointer,
                frames: self.frames,
                _lifetime: PhantomData,
            }),
            None => None,
        }
    }

    /// Iterates over each output frame exactly once.
    pub fn frames_mut(&mut self) -> StereoFramesMut<'_> {
        StereoFramesMut {
            input: self.input,
            output: self.output,
            next: 0,
            frames: self.frames,
            _lifetime: PhantomData,
        }
    }
}

fn ranges_overlap(first: usize, second: usize, bytes: usize) -> Result<bool, BufferError> {
    let first_end = first.checked_add(bytes).ok_or(BufferError::SizeOverflow)?;
    let second_end = second.checked_add(bytes).ok_or(BufferError::SizeOverflow)?;
    Ok(first < second_end && second < first_end)
}

/// Copy-only view of an interleaved stereo input buffer.
#[derive(Clone, Copy)]
pub struct StereoInput<'audio> {
    pointer: *const f32,
    frames: usize,
    _lifetime: PhantomData<&'audio f32>,
}

impl<'audio> StereoInput<'audio> {
    pub const fn len(self) -> usize {
        self.frames
    }

    pub const fn is_empty(self) -> bool {
        self.frames == 0
    }

    pub fn frames(self) -> StereoInputFrames<'audio> {
        StereoInputFrames {
            pointer: self.pointer,
            next: 0,
            frames: self.frames,
            _lifetime: PhantomData,
        }
    }
}

/// Iterator yielding copied raw-input frames.
pub struct StereoInputFrames<'audio> {
    pointer: *const f32,
    next: usize,
    frames: usize,
    _lifetime: PhantomData<&'audio f32>,
}

impl Iterator for StereoInputFrames<'_> {
    type Item = [f32; CHANNELS];

    fn next(&mut self) -> Option<Self::Item> {
        if self.next == self.frames {
            return None;
        }
        let offset = self.next * CHANNELS;
        self.next += 1;

        // SAFETY: construction established a readable interleaved input range.
        // `offset` identifies one in-bounds frame and values are copied, so no
        // reference aliases the render output.
        Some(unsafe {
            [
                ptr::read(self.pointer.add(offset)),
                ptr::read(self.pointer.add(offset + 1)),
            ]
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.frames - self.next;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for StereoInputFrames<'_> {}

/// Mutable stereo frame iterator. Each item owns one unique output location.
pub struct StereoFramesMut<'buffer> {
    input: *const f32,
    output: *mut f32,
    next: usize,
    frames: usize,
    _lifetime: PhantomData<&'buffer mut f32>,
}

impl<'buffer> Iterator for StereoFramesMut<'buffer> {
    type Item = StereoFrame<'buffer>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next == self.frames {
            return None;
        }
        let offset = self.next * CHANNELS;
        self.next += 1;

        // SAFETY: buffer construction established readable input for every
        // frame. Both values are copied before the caller can mutate output,
        // which also makes exact in-place operation well-defined.
        let input = unsafe {
            [
                ptr::read(self.input.add(offset)),
                ptr::read(self.input.add(offset + 1)),
            ]
        };
        // SAFETY: construction established writable output for every frame.
        // Each monotonically increasing offset is yielded only once.
        let output = unsafe { self.output.add(offset) };

        Some(StereoFrame {
            input,
            output,
            _lifetime: PhantomData,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.frames - self.next;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for StereoFramesMut<'_> {}

/// One copied stereo input frame and its unique output destination.
pub struct StereoFrame<'buffer> {
    input: [f32; CHANNELS],
    output: *mut f32,
    _lifetime: PhantomData<&'buffer mut f32>,
}

impl StereoFrame<'_> {
    pub const fn input(&self) -> [f32; CHANNELS] {
        self.input
    }

    /// Writes both output channels for this uniquely yielded frame.
    pub fn write(&mut self, output: [f32; CHANNELS]) {
        // SAFETY: the iterator gave this frame one unique in-bounds output
        // destination, and no other safe frame can address it.
        unsafe {
            ptr::write(self.output, output[0]);
            ptr::write(self.output.add(1), output[1]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: *const f32, output: *mut f32, frames: usize) {
        // SAFETY: every caller supplies valid storage for `frames` stereo
        // samples, either separate or exactly in place.
        let mut buffer =
            unsafe { StereoBuffer::from_raw(input, output, ptr::null(), frames) }.unwrap();
        for mut frame in buffer.frames_mut() {
            let [left, right] = frame.input();
            frame.write([left * 0.5 + right, right * -0.25 - left]);
        }
    }

    fn expected(input: &[f32]) -> std::vec::Vec<f32> {
        input
            .chunks_exact(2)
            .flat_map(|frame| [frame[0] * 0.5 + frame[1], frame[1] * -0.25 - frame[0]])
            .collect()
    }

    #[test]
    fn separate_and_in_place_match_with_untouched_guards() {
        let mut seed = 0x3141_5926_u32;
        let mut counts = std::vec![0, 1, 3, 127, u16::MAX as usize];
        for _ in 0..24 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            counts.push((seed % 1024) as usize);
        }

        for frames in counts {
            let samples = frames * CHANNELS;
            let source: std::vec::Vec<f32> = (0..samples)
                .map(|index| ((index as i32 % 257) - 128) as f32 / 128.0)
                .collect();
            let wanted = expected(&source);

            let guard = f32::from_bits(0x4b12_3456);
            let mut separate = std::vec![guard; samples + 8];
            render(source.as_ptr(), separate[4..].as_mut_ptr(), frames);
            assert_eq!(&separate[..4], &[guard; 4]);
            assert_eq!(&separate[4..4 + samples], wanted.as_slice());
            assert_eq!(&separate[4 + samples..], &[guard; 4]);

            let mut in_place = std::vec![guard; samples + 8];
            in_place[4..4 + samples].copy_from_slice(&source);
            let pointer = in_place[4..].as_mut_ptr();
            render(pointer.cast_const(), pointer, frames);
            assert_eq!(&in_place[..4], &[guard; 4]);
            assert_eq!(&in_place[4..4 + samples], wanted.as_slice());
            assert_eq!(&in_place[4 + samples..], &[guard; 4]);
        }
    }

    #[test]
    fn frame_input_is_cached_before_in_place_output_write() {
        let mut samples = [1.0, 2.0];
        // SAFETY: one exact in-place stereo frame is live for the buffer.
        let mut buffer = unsafe {
            StereoBuffer::from_raw(samples.as_ptr(), samples.as_mut_ptr(), ptr::null(), 1)
        }
        .unwrap();
        let mut frame = buffer.frames_mut().next().unwrap();
        assert_eq!(frame.input(), [1.0, 2.0]);
        frame.write([9.0, 8.0]);
        assert_eq!(samples, [9.0, 8.0]);
    }

    #[test]
    fn raw_input_is_copy_only_and_optional() {
        let input = [1.0, 2.0, 3.0, 4.0];
        let raw = [5.0, 6.0, 7.0, 8.0];
        let mut output = [0.0; 4];
        // SAFETY: all arrays hold two stereo frames and are disjoint.
        let buffer =
            unsafe { StereoBuffer::from_raw(input.as_ptr(), output.as_mut_ptr(), raw.as_ptr(), 2) }
                .unwrap();
        assert_eq!(
            buffer
                .raw_input()
                .unwrap()
                .frames()
                .collect::<std::vec::Vec<_>>(),
            [[5.0, 6.0], [7.0, 8.0]]
        );
    }

    #[test]
    fn rejects_null_and_partial_overlap_without_dereferencing() {
        // SAFETY: invalid pointers are rejected before any sample access.
        assert_eq!(
            unsafe { StereoBuffer::from_raw(ptr::null(), ptr::null_mut(), ptr::null(), 1) }.err(),
            Some(BufferError::NullPointer)
        );

        let mut samples = [0.0; 8];
        // SAFETY: both ranges lie within `samples`; their forbidden partial
        // overlap is detected during construction in test/debug mode.
        assert_eq!(
            unsafe {
                StereoBuffer::from_raw(
                    samples.as_ptr(),
                    samples.as_mut_ptr().add(2),
                    ptr::null(),
                    2,
                )
            }
            .err(),
            Some(BufferError::PartialOverlap)
        );

        // Zero frames never dereference pointers, including null pointers.
        // SAFETY: no storage is required for an empty buffer.
        let empty = unsafe { StereoBuffer::from_raw(ptr::null(), ptr::null_mut(), ptr::null(), 0) }
            .unwrap();
        assert!(empty.is_empty());
    }
}
