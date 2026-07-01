use core::{
    cell::Cell,
    marker::{PhantomData, PhantomPinned},
    pin::Pin,
    ptr::null,
};

use crate::lending;

#[derive(Debug)]
pub struct Links {
    prev: Cell<*const Links>,
    next: Cell<*const Links>,
    _pin: PhantomPinned,
}

impl Links {
    /// ## Safety:
    /// The returned object is in an uninitialized state.
    /// It can be initialized via [`Links::init`] or [`Links::link`].
    pub const fn new() -> Self {
        Self {
            prev: Cell::new(null()),
            next: Cell::new(null()),
            _pin: PhantomPinned,
        }
    }

    pub fn init(self: Pin<&Self>) {
        let ptr = &raw const *self;
        assert!(self.prev.get() == null());
        assert!(self.next.get() == null());
        self.prev.replace(ptr);
        self.next.replace(ptr);
    }

    pub fn prev(self: *const Self) -> *const Self {
        unsafe { (*self).prev.get() }
    }

    fn set_prev(self: *const Self, prev: *const Self) {
        unsafe {
            (*self).prev.replace(prev);
        }
    }

    pub fn next(self: *const Self) -> *const Self {
        unsafe { (*self).next.get() }
    }

    fn set_next(self: *const Self, next: *const Self) {
        unsafe {
            (*self).next.replace(next);
        }
    }

    pub fn unlink(self: *const Self) {
        let prev = self.prev();
        let next = self.next();
        assert!(next != null());
        if next != self {
            prev.set_next(next);
            next.set_prev(prev);
            self.set_prev(self);
            self.set_next(self);
        }
    }

    pub fn link(self: *const Self, head: *const Self) {
        let next = self.next();
        assert!(next == self || next == null());
        assert!(head != null());
        assert!(head != self);
        let next = head.next();
        assert!(next != null());
        self.set_prev(head);
        self.set_next(next);
        head.set_next(self);
        next.set_prev(self);
    }

    pub fn is_linked(self: *const Self) -> bool {
        let next = self.next();
        assert!(next != null());
        self != next
    }

    pub fn remove(self: Pin<&Self>) {
        (&raw const *self).unlink();
    }
}

impl Default for Links {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for Links {}

pub unsafe trait Adapter<T> {
    fn from_links(links: *const Links) -> *const T;
    fn to_links(obj: *const T) -> *const Links;
}

/// ## Safety:
/// * Should be implemented for list items
/// that are effectively owned by the containing list.
/// This permits mutable iteration via the list.
/// * This precludes items that have multiple embedded lists unless
/// caution is taken to prevent multiple simultaneous iterators.
pub unsafe trait ListOwned {}

pub struct List<T, A: Adapter<T>> {
    sentinel: Links,
    _adapter: PhantomData<A>,
    _items: PhantomData<T>,
}

impl<T, A: Adapter<T>> List<T, A> {
    pub const fn new() -> Self {
        Self {
            sentinel: Links::new(),
            _adapter: PhantomData,
            _items: PhantomData,
        }
    }

    pub fn init(self: Pin<&Self>) {
        unsafe { Pin::new_unchecked(&self.sentinel).init() };
    }

    /// ## Safety:
    /// The caller must ensure that obj lives as long
    /// as the list or is appropriately removed on drop.
    pub fn add(self: Pin<&Self>, obj: Pin<&T>) {
        let links = A::to_links(&raw const *obj);
        links.link(&raw const self.sentinel);
    }

    /// ## Safety:
    /// See [`List::add`].
    pub fn add_tail(self: Pin<&Self>, obj: Pin<&T>) {
        let links = A::to_links(&raw const *obj);
        links.link((&raw const self.sentinel).prev());
    }

    pub fn is_empty(self: Pin<&Self>) -> bool {
        !(&raw const self.sentinel).is_linked()
    }

    pub fn iter<'a>(self: Pin<&'a Self>) -> Iter<'a, T, A> {
        let head = &raw const self.sentinel;
        let next = head.next();
        assert!(next != null());
        Iter {
            list: self,
            next,
            _adapter: PhantomData,
            _items: PhantomData,
        }
    }
}

impl<T: ListOwned, A: Adapter<T>> List<T, A> {
    pub fn iter_mut<'a>(self: Pin<&'a mut Self>) -> IterMut<'a, T, A> {
        let head = &raw const self.sentinel;
        let next = head.next();
        assert!(next != null());
        IterMut {
            list: self,
            next,
            _adapter: PhantomData,
            _items: PhantomData,
        }
    }
}

impl<T, A: Adapter<T>> Default for List<T, A> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Iter<'a, T, A: Adapter<T>> {
    list: Pin<&'a List<T, A>>,
    next: *const Links,
    _adapter: PhantomData<A>,
    _items: PhantomData<&'a T>,
}

impl<'a, T, A: Adapter<T>> Iterator for Iter<'a, T, A> {
    type Item = Pin<&'a T>;

    fn next(&mut self) -> Option<Self::Item> {
        if &raw const self.list.sentinel == self.next {
            None
        } else {
            let next = self.next;
            self.next = next.next();
            unsafe {
                let obj = &*A::from_links(next);
                Some(Pin::new_unchecked(obj))
            }
        }
    }
}

pub struct IterMut<'a, T, A: Adapter<T>> {
    list: Pin<&'a mut List<T, A>>,
    next: *const Links,
    _adapter: PhantomData<A>,
    _items: PhantomData<&'a T>,
}

impl<'a, T, A: Adapter<T>> lending::Iterator for IterMut<'a, T, A> {
    type Item<'b>
        = Pin<&'b mut T>
    where
        Self: 'b;

    fn next<'b>(&'b mut self) -> Option<Self::Item<'b>> {
        if &raw const self.list.sentinel == self.next {
            None
        } else {
            let next = self.next;
            self.next = next.next();
            unsafe {
                let obj = &mut *A::from_links(next).cast_mut();
                Some(Pin::new_unchecked(obj))
            }
        }
    }
}
