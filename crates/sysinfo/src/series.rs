use util::HISTORY;

#[derive(Clone, Debug)]
pub struct Series<T: Copy + Default> {
    inner: Box<[T; HISTORY]>,
    last: usize,
}
impl<T: Copy + Default> Default for Series<T> {
    fn default() -> Self {
        Self {
            inner: Box::new([T::default(); HISTORY]),
            last: HISTORY - 1,
        }
    }
}
impl<T: Copy + Default> Series<T> {
    pub fn push(&mut self, item: T) {
        self.last += 1;
        if self.last == HISTORY {
            self.last = 0;
        }
        self.inner[self.last] = item;
    }
    pub fn capacity() -> usize {
        HISTORY
    }
    pub fn latest(&self) -> T {
        self.inner[self.last]
    }
    pub fn iter<'a>(&'a self) -> impl 'a + Iterator<Item = T> {
        Iterator::chain(
            self.inner[(self.last + 1)..].iter().copied(),
            self.inner[..(self.last + 1)].iter().copied(),
        )
    }
    pub fn chunks<'a>(
        &'a self,
        chunk_size: usize,
    ) -> (&'a [T], impl Iterator<Item = &'a [T]>, &'a [T]) {
        let tail = self.inner[(self.last + 1)..].rchunks_exact(chunk_size);
        let head = self.inner[..(self.last + 1)].chunks_exact(chunk_size);
        let first_chunk = tail.remainder();
        let last_chunk = head.remainder();
        let iterator = tail.rev().chain(head);
        (first_chunk, iterator, last_chunk)
    }
}
