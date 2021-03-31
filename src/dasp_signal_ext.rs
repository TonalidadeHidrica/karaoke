use dasp::Frame;
use dasp::Signal;

pub trait SignalExt: Signal {
    fn multiplexed(self, n_channel: usize) -> Multiplexed<Self>
    where
        Self: Sized,
    {
        assert!(n_channel > 0);
        Multiplexed {
            n_channel,
            i: 0,
            last: Self::Frame::EQUILIBRIUM,
            signal: self,
        }
    }
}

impl<S> SignalExt for S where S: Signal + ?Sized {}

pub struct Multiplexed<S>
where
    S: Signal,
{
    n_channel: usize,
    i: usize,
    last: S::Frame,
    signal: S,
}

impl<S> Signal for Multiplexed<S>
where
    S: Signal,
{
    type Frame = S::Frame;

    fn next(&mut self) -> Self::Frame {
        if self.i == 0 {
            self.i = self.n_channel;
            self.last = self.signal.next();
        }
        self.i -= 1;
        self.last
    }

    fn is_exhausted(&self) -> bool {
        self.i == 0 && self.signal.is_exhausted()
    }
}
