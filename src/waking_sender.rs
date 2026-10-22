use std::sync::Arc;

pub struct WakingSender<T> {
    waker: Arc<mio::Waker>,
    sender: crossbeam_channel::Sender<T>,
}

impl<T> Clone for WakingSender<T> {
    fn clone(&self) -> Self {
        Self {
            waker: self.waker.clone(),
            sender: self.sender.clone(),
        }
    }
}

impl<T> WakingSender<T> {
    pub fn new(waker: Arc<mio::Waker>, sender: crossbeam_channel::Sender<T>) -> Self {
        Self { waker, sender }
    }

    pub fn send(&self, msg: T) -> Result<(), crossbeam_channel::SendError<T>> {
        self.sender.send(msg)?;
        let _ = self.waker.wake();
        Ok(())
    }

    #[allow(dead_code)]
    pub fn try_send(&self, msg: T) -> Result<(), crossbeam_channel::TrySendError<T>> {
        self.sender.try_send(msg)?;
        self.waker.wake().unwrap();
        Ok(())
    }
}

#[allow(dead_code)]
pub struct WakingOneshot<T> {
    waker: Arc<mio::Waker>,
    sender: crossbeam_channel::Sender<T>,
}

#[allow(dead_code)]
impl<T> WakingOneshot<T> {
    pub fn new(waker: Arc<mio::Waker>, sender: crossbeam_channel::Sender<T>) -> Self {
        Self { waker, sender }
    }

    pub fn send(self, msg: T) -> Result<(), crossbeam_channel::SendError<T>> {
        self.sender.send(msg)?;
        self.waker.wake().unwrap();
        Ok(())
    }
}
