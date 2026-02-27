use std::collections::VecDeque;

#[derive(Clone, serde::Serialize)]
pub struct RingBuffer<T>
where
    T: Clone,
{
    container: VecDeque<T>,
    capacity: usize,
}

pub struct RingBufferIntoIterator<T>
where
    T: Clone,
{
    inner: RingBuffer<T>,
}

pub struct RingBufferIterator<'a, T>
where
    T: Clone,
{
    inner: &'a RingBuffer<T>,
    index: usize,
}

impl<T> RingBuffer<T>
where
    T: Clone,
{
    pub fn new(capacity: usize) -> RingBuffer<T> {
        RingBuffer {
            container: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, value: T) {
        if self.container.len() >= self.capacity {
            self.container.pop_back();
        }
        self.container.push_front(value);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.container.pop_front()
    }

    pub fn len(&self) -> usize {
        self.container.len()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.container.get(index)
    }
}

impl<T> IntoIterator for RingBuffer<T>
where
    T: Clone,
{
    type Item = T;
    type IntoIter = RingBufferIntoIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        RingBufferIntoIterator { inner: self }
    }
}

impl<T> Iterator for RingBufferIntoIterator<T>
where
    T: Clone,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.pop()
    }
}

impl<'a, T> IntoIterator for &'a RingBuffer<T>
where
    T: Clone,
{
    type Item = T;
    type IntoIter = RingBufferIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        RingBufferIterator { inner: self, index: 0 }
    }
}

impl<'a, T> Iterator for RingBufferIterator<'a, T>
where
    T: Clone,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.index >= self.inner.len() {
            None
        } else {
            let result = self.inner.get(self.index).map(|e| e.clone());
            self.index += 1;
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut buf: RingBuffer<u64> = RingBuffer::new(2);
        buf.push(1);
        buf.push(2);
        buf.push(3);
        let mut iter = buf.into_iter();
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_non_consuming() {
        let mut buf: RingBuffer<u64> = RingBuffer::new(2);
        buf.push(1);
        buf.push(2);
        buf.push(3);
        let mut iter = (&buf).into_iter();
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), None);
        let mut iter2 = (&buf).into_iter();
        assert_eq!(iter2.next(), Some(3));
        assert_eq!(iter2.next(), Some(2));
        assert_eq!(iter2.next(), None);
    }
}
