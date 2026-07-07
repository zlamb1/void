use core::{
    marker::PhantomData,
    pin::{Pin, UnsafePinned},
    ptr::null,
};

mod raw {
    use core::{cell::UnsafeCell, marker::PhantomPinned, ptr::null};

    #[derive(Debug)]
    pub struct Raw {
        prev: UnsafeCell<*const Self>,
        next: UnsafeCell<*const Self>,
        #[cfg(debug_assertions)]
        sentinel: UnsafeCell<*const Self>,
        _pin: PhantomPinned,
    }

    impl Raw {
        pub const fn new() -> Self {
            Self {
                prev: UnsafeCell::new(null()),
                next: UnsafeCell::new(null()),
                #[cfg(debug_assertions)]
                sentinel: UnsafeCell::new(null()),
                _pin: PhantomPinned,
            }
        }

        pub unsafe fn init(this: *const Self) {
            assert_ne!(this, null());
            assert_eq!(unsafe { Self::prev(this) }, null());
            assert_eq!(unsafe { Self::next(this) }, null());
            Self::set_prev(this, this);
            Self::set_next(this, this);
        }

        pub unsafe fn prev(this: *const Self) -> *const Self {
            debug_assert_ne!(this, null());
            unsafe { *UnsafeCell::raw_get(&raw const (*this).prev) }
        }

        pub unsafe fn next(this: *const Self) -> *const Self {
            debug_assert_ne!(this, null());
            unsafe { *UnsafeCell::raw_get(&raw const (*this).next) }
        }

        pub unsafe fn is_init(this: *const Self) -> bool {
            debug_assert_ne!(this, null());
            unsafe { Self::prev(this) != null() && Self::next(this) != null() }
        }

        #[cfg(debug_assertions)]
        fn sentinel(this: *const Self) -> *const Self {
            unsafe { *UnsafeCell::raw_get(&raw const (*this).sentinel) }
        }

        fn set_prev(this: *const Self, new_prev: *const Self) {
            unsafe {
                *UnsafeCell::raw_get(&raw const (*this).prev) = new_prev;
            }
        }

        fn set_next(this: *const Self, new_next: *const Self) {
            unsafe {
                *UnsafeCell::raw_get(&raw const (*this).next) = new_next;
            }
        }

        #[cfg(debug_assertions)]
        fn set_sentinel(this: *const Self, new_sentinel: *const Self) {
            unsafe {
                *UnsafeCell::raw_get(&raw const (*this).sentinel) = new_sentinel;
            }
        }

        pub unsafe fn link(this: *const Self, head: *const Self, sentinel: *const Self) {
            debug_assert_ne!(this, null());
            debug_assert_ne!(head, null());
            debug_assert_ne!(sentinel, null());
            debug_assert_eq!(Self::sentinel(this), null());
            let (prev, next) = unsafe { (Self::prev(this), Self::next(this)) };
            debug_assert!(prev == next && (next == null() || next == this));
            let (prev, next) = unsafe { (head, Self::next(head)) };
            debug_assert_ne!(next, null());
            Self::set_prev(this, prev);
            Self::set_next(this, next);
            Self::set_next(prev, this);
            Self::set_prev(next, this);
            #[cfg(debug_assertions)]
            Self::set_sentinel(this, sentinel);
        }

        pub unsafe fn unlink(this: *const Self, sentinel: *const Self) {
            debug_assert_ne!(this, null());
            debug_assert_eq!(Self::sentinel(this), sentinel);
            let (prev, next) = unsafe { (Self::prev(this), Self::next(this)) };
            debug_assert_ne!(prev, null());
            debug_assert_ne!(next, null());
            Self::set_next(prev, next);
            Self::set_prev(next, prev);
            Self::set_prev(this, this);
            Self::set_next(this, this);
            #[cfg(debug_assertions)]
            Self::set_sentinel(this, null());
        }

        pub unsafe fn is_terminal(this: *const Self) -> bool {
            debug_assert_ne!(this, null());
            let (prev, next) = unsafe { (Self::prev(this), Self::next(this)) };
            prev == next && (next == null() || next == this)
        }
    }

    impl Drop for Raw {
        fn drop(&mut self) {
            let this = &raw const *self;
            let (prev, next) = unsafe { (Self::prev(this), Self::next(this)) };
            assert!(prev == next && (next == null() || next == this));
        }
    }

    unsafe impl Send for Raw {}
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Links(UnsafePinned<raw::Raw>);

impl Links {
    pub const fn new() -> Self {
        Self(UnsafePinned::new(raw::Raw::new()))
    }
}

impl Default for Links {
    fn default() -> Self {
        Self::new()
    }
}

pub unsafe trait Adapter<T> {
    unsafe fn from_links(links: *const Links) -> *const T;
    unsafe fn to_links(obj: *const T) -> *const Links;
}

#[derive(Debug)]
pub struct List<T, A: Adapter<T>> {
    links: Links,
    len: usize,
    _items: PhantomData<T>,
    _adapter: PhantomData<A>,
}

impl<T, A: Adapter<T>> List<T, A> {
    pub const fn new() -> Self {
        Self {
            links: Links::new(),
            len: 0,
            _items: PhantomData,
            _adapter: PhantomData,
        }
    }

    fn raw_sentinel(self: Pin<&Self>) -> *const raw::Raw {
        self.links.0.get().cast_const()
    }

    fn raw_sentinel_mut(self: Pin<&mut Self>) -> *const raw::Raw {
        self.as_ref().raw_sentinel()
    }

    pub fn init(self: Pin<&mut Self>) {
        unsafe { raw::Raw::init(self.raw_sentinel_mut()) };
    }

    pub fn len(self: Pin<&Self>) -> usize {
        self.len
    }

    pub fn is_init(self: Pin<&Self>) -> bool {
        unsafe { raw::Raw::is_init(self.raw_sentinel()) }
    }

    pub fn is_empty(self: Pin<&Self>) -> bool {
        debug_assert!(self.is_init());
        debug_assert_eq!(self.len == 0, unsafe {
            raw::Raw::is_terminal(self.raw_sentinel())
        });
        self.len == 0
    }

    pub fn clear(mut self: Pin<&mut Self>) {
        let raw_sentinel = self.as_mut().raw_sentinel_mut();
        unsafe {
            let mut current = raw::Raw::next(raw_sentinel);
            while current != raw_sentinel {
                let next = raw::Raw::next(current);
                raw::Raw::unlink(current, raw_sentinel);
                current = next;
            }
            self.get_unchecked_mut().len = 0;
        }
    }

    unsafe fn item<'a>(raw: *const raw::Raw) -> Pin<&'a T> {
        unsafe { Pin::new_unchecked(&*A::from_links(raw.cast())) }
    }

    pub fn front<'a>(self: Pin<&'a Self>) -> Option<Pin<&'a T>> {
        if self.is_empty() {
            None
        } else {
            let obj = unsafe { List::<T, A>::item::<'a>(raw::Raw::next(self.raw_sentinel())) };
            Some(obj)
        }
    }

    pub fn back<'a>(self: Pin<&'a Self>) -> Option<Pin<&'a T>> {
        if self.is_empty() {
            None
        } else {
            let obj = unsafe { List::<T, A>::item::<'a>(raw::Raw::prev(self.raw_sentinel())) };
            Some(obj)
        }
    }

    fn push(mut self: Pin<&mut Self>, obj: Pin<&T>, head: *const raw::Raw) {
        unsafe {
            let links = UnsafePinned::raw_get(&raw const (*A::to_links(&raw const *obj)).0);
            raw::Raw::link(links, head, self.as_mut().raw_sentinel_mut());
            self.get_unchecked_mut().len += 1;
        }
    }

    /// ## Safety:
    /// The object T must live for at least as long as it resides in the
    /// list. Each object T _must_ be removed before its drop handler runs.
    /// Otherwise aliasing violations _will_ occur, even if access is serialized.
    /// Additionally, the link referenced by the adapter must not be linked into
    /// another list right before this method is called.
    pub unsafe fn push_front(mut self: Pin<&mut Self>, obj: Pin<&T>) {
        let head = self.as_mut().raw_sentinel_mut();
        self.push(obj, head);
    }

    /// ## Safety:
    /// See [`List::push_front`].
    pub unsafe fn push_back(mut self: Pin<&mut Self>, obj: Pin<&T>) {
        let head = unsafe { raw::Raw::prev(self.as_mut().raw_sentinel_mut()) };
        self.push(obj, head);
    }

    fn pop<'a>(mut self: Pin<&'a mut Self>, raw: *const raw::Raw) -> Pin<&'a T> {
        debug_assert!(raw != null());
        unsafe {
            let obj = List::<T, A>::item::<'a>(raw);
            raw::Raw::unlink(raw, self.as_mut().raw_sentinel_mut());
            self.get_unchecked_mut().len -= 1;
            obj
        }
    }

    pub fn pop_front(mut self: Pin<&mut Self>) -> Option<Pin<&T>> {
        if self.as_ref().is_empty() {
            None
        } else {
            let raw_sentinel = self.as_mut().raw_sentinel_mut();
            unsafe { Some(self.pop(raw::Raw::next(raw_sentinel))) }
        }
    }

    pub fn pop_back(mut self: Pin<&mut Self>) -> Option<Pin<&T>> {
        if self.as_ref().is_empty() {
            None
        } else {
            let raw_sentinel = self.as_mut().raw_sentinel_mut();
            unsafe { Some(self.pop(raw::Raw::prev(raw_sentinel))) }
        }
    }

    pub fn cursor<'a>(self: Pin<&'a Self>) -> Cursor<'a, T, A> {
        Cursor::new(self)
    }

    pub fn iter<'a>(self: Pin<&'a Self>) -> Iter<'a, T, A> {
        Iter::new(self)
    }

    /// ## Safety:
    /// The object T must have been previously added
    /// to this list and still reside within it.
    pub unsafe fn remove(mut self: Pin<&mut Self>, obj: Pin<&T>) {
        debug_assert!(self.len > 0);
        unsafe {
            let links = UnsafePinned::raw_get(&raw const (*A::to_links(&raw const *obj)).0);
            raw::Raw::unlink(links, self.as_mut().raw_sentinel_mut());
            self.get_unchecked_mut().len -= 1;
        }
    }
}

impl<T, A: Adapter<T>> Default for List<T, A> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Cursor<'a, T, A: Adapter<T>> {
    list: Pin<&'a List<T, A>>,
    current: *const raw::Raw,
}

impl<'a, T, A: Adapter<T>> Cursor<'a, T, A> {
    fn new(list: Pin<&'a List<T, A>>) -> Self {
        assert!(list.is_init());
        let current = unsafe { raw::Raw::next(list.raw_sentinel()) };
        Self { list, current }
    }

    pub fn at_sentinel(&self) -> bool {
        self.current == self.list.raw_sentinel()
    }

    pub fn current(&self) -> Option<Pin<&'a T>> {
        if self.at_sentinel() {
            None
        } else {
            let obj = unsafe { List::<T, A>::item::<'a>(self.current) };
            Some(obj)
        }
    }

    pub fn prev(&mut self) {
        self.current = unsafe { raw::Raw::prev(self.current) };
    }

    pub fn next(&mut self) {
        self.current = unsafe { raw::Raw::next(self.current) };
    }
}

impl<'a, T, A: Adapter<T>> IntoIterator for Pin<&'a List<T, A>> {
    type Item = Pin<&'a T>;
    type IntoIter = Iter<'a, T, A>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Debug)]
pub struct Iter<'a, T, A: Adapter<T>> {
    cursor: Cursor<'a, T, A>,
}

impl<'a, T, A: Adapter<T>> Iter<'a, T, A> {
    fn new(list: Pin<&'a List<T, A>>) -> Self {
        Self {
            cursor: Cursor::new(list),
        }
    }
}

impl<'a, T, A: Adapter<T>> Iterator for Iter<'a, T, A> {
    type Item = Pin<&'a T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.current().inspect(|_| self.cursor.next())
    }
}
