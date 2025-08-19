use core::fmt;

use abs_path::AbsPathBuf;
use fs::walk::FsExt;
use futures_lite::{StreamExt, future};

#[test]
fn paths_simple() {
    future::block_on(async {
        let fs = mock::fs! {
            "foo": {
                "bar.txt": "",
                "baz.txt": "",
            },
        };

        let paths = fs
            .walk(&fs.root())
            .paths()
            .map(Result::unwrap)
            .collect::<Paths>()
            .await;

        assert_eq!(paths, ["/foo", "/foo/bar.txt", "/foo/baz.txt",]);
    });
}

#[derive(Default)]
struct Paths {
    inner: std::collections::HashSet<AbsPathBuf>,
}

impl fmt::Debug for Paths {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set()
            .entries(self.inner.iter().map(AsRef::<str>::as_ref))
            .finish()
    }
}

impl<Path: Into<AbsPathBuf>> FromIterator<Path> for Paths {
    fn from_iter<T: IntoIterator<Item = Path>>(iter: T) -> Self {
        let mut this = Self::default();
        this.extend(iter);
        this
    }
}

impl<Path: Into<AbsPathBuf>> Extend<Path> for Paths {
    fn extend<T: IntoIterator<Item = Path>>(&mut self, iter: T) {
        self.inner.extend(iter.into_iter().map(Into::into));
    }
}

impl<Iter, Path> PartialEq<Iter> for Paths
where
    Iter: IntoIterator<Item = Path> + Clone,
    Path: AsRef<str>,
{
    fn eq(&self, other: &Iter) -> bool {
        let mut num_checked = 0;
        for path in other.clone().into_iter() {
            if !self.inner.contains(path.as_ref()) {
                return false;
            }
            num_checked += 1;
        }
        num_checked == self.inner.len()
    }
}
