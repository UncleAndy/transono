use rubato::audioadapter::{Adapter, AdapterMut};

pub struct PlanarAdapter<'a, T> {
    channels: &'a mut [Vec<T>],
}

impl<'a, T> PlanarAdapter<'a, T> {
    pub fn new(
        channels: &'a mut [Vec<T>],
    ) -> Self {
        debug_assert!(
            channels
                .windows(2)
                .all(|w| w[0].len() == w[1].len())
        );

        Self { channels }
    }
}

unsafe impl<'a, T> Adapter<'a, T> for PlanarAdapter<'a, T>
where
    T: Copy
{
    #[inline(always)]
    unsafe fn read_sample_unchecked(
        &self,
        channel: usize,
        frame: usize,
    ) -> T {
        let channel = self.channels.get_unchecked(channel);

        *channel.as_ptr().add(frame)
    }

    fn channels(&self) -> usize {
        self.channels.len()
    }

    fn frames(&self) -> usize {
        self.channels
            .first()
            .map_or(0, Vec::len)
    }
}

unsafe impl<'a, T> AdapterMut<'a, T> for PlanarAdapter<'a, T>
where
    T: Copy + Clone
{
    #[inline(always)]
    unsafe fn write_sample_unchecked(
        &mut self,
        channel: usize,
        frame: usize,
        value: &T,
    ) -> bool {
        let channel = self.channels.get_unchecked_mut(channel);

        *channel.as_mut_ptr().add(frame) = *value;

        false
    }
}
