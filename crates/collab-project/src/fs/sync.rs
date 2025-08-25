use collab_types::puff;
use puff::node::{Deleted, Visible};

use crate::abs_path::{AbsPathBuf, NodeName};
use crate::project::{State, StateMut};
use crate::{fs, text};

/// TODO: docs.
pub struct SyncActions<'a> {
    inner: fs::PuffSyncActions<'a>,
    state: StateMut<'a>,
}

/// TODO: docs.
pub enum SyncAction<'a> {
    /// TODO: docs.
    Create(Create<'a>),

    /// TODO: docs.
    CreateAndResolve(CreateAndResolve<'a>),

    /// TODO: docs.
    Delete(Delete<'a>),

    /// TODO: docs.
    Move(Move<'a>),

    /// TODO: docs.
    MoveAndResolve(MoveAndResolve<'a>),

    /// TODO: docs.
    Rename(Rename<'a>),

    /// TODO: docs.
    RenameAndResolve(RenameAndResolve<'a>),
}

/// TODO: docs.
pub struct Create<'a> {
    inner: fs::PuffCreate<'a>,
    state: State<'a>,
}

/// TODO: docs.
pub struct CreateAndResolve<'a> {
    inner: fs::PuffCreateAndResolve<'a>,
    state: StateMut<'a>,
}

/// TODO: docs.
pub struct Delete<'a> {
    inner: fs::PuffDelete<'a>,
    state: State<'a>,
}

/// TODO: docs.
pub struct Move<'a> {
    inner: fs::PuffMove<'a>,
    state: State<'a>,
}

/// TODO: docs.
pub struct MoveAndResolve<'a> {
    inner: fs::PuffMoveAndResolve<'a>,
    state: StateMut<'a>,
}

/// TODO: docs.
pub struct Rename<'a> {
    inner: fs::PuffRename<'a>,
    state: State<'a>,
}

/// TODO: docs.
pub struct RenameAndResolve<'a> {
    inner: fs::PuffRenameAndResolve<'a>,
    state: StateMut<'a>,
}

/// TODO: docs.
pub struct ResolveConflict<'a> {
    inner: fs::PuffResolveConflict<'a>,
    state: StateMut<'a>,
}

impl<'a> SyncActions<'a> {
    /// TODO: docs.
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn next(&mut self) -> Option<SyncAction<'_>> {
        self.inner.next().map(|action| match action {
            fs::PuffSyncAction::Create(mut inner) => {
                if let fs::PuffNodeMut::File(mut file_mut) = inner.node_mut() {
                    integrate_backlogged_edits(&mut file_mut, &mut self.state);
                    integrate_backlogged_annotations(
                        file_mut.as_file(),
                        self.state.text_ctx_mut(),
                    );
                }
                SyncAction::Create(fs::Create {
                    inner,
                    state: self.state.as_ref(),
                })
            },
            fs::PuffSyncAction::CreateAndResolve(mut inner) => {
                if let fs::PuffNodeMut::File(mut file_mut) =
                    inner.create().node_mut()
                {
                    integrate_backlogged_edits(&mut file_mut, &mut self.state);
                    integrate_backlogged_annotations(
                        file_mut.as_file(),
                        self.state.text_ctx_mut(),
                    );
                }
                SyncAction::CreateAndResolve(CreateAndResolve {
                    inner,
                    state: self.state.reborrow(),
                })
            },
            fs::PuffSyncAction::Delete(inner) => SyncAction::Delete(Delete {
                inner,
                state: self.state.as_ref(),
            }),
            fs::PuffSyncAction::Move(inner) => {
                SyncAction::Move(Move { inner, state: self.state.as_ref() })
            },
            fs::PuffSyncAction::MoveAndResolve(inner) => {
                SyncAction::MoveAndResolve(MoveAndResolve {
                    inner,
                    state: self.state.reborrow(),
                })
            },
            fs::PuffSyncAction::Rename(inner) => SyncAction::Rename(Rename {
                inner,
                state: self.state.as_ref(),
            }),
            fs::PuffSyncAction::RenameAndResolve(inner) => {
                SyncAction::RenameAndResolve(RenameAndResolve {
                    inner,
                    state: self.state.reborrow(),
                })
            },
        })
    }

    #[inline]
    pub(crate) fn new(
        inner: fs::PuffSyncActions<'a>,
        state: StateMut<'a>,
    ) -> Self {
        Self { inner, state }
    }
}

fn integrate_backlogged_edits(
    file: &mut fs::PuffFileMut<'_, Visible>,
    state: &mut StateMut<'_>,
) {
    let global_id = file.global_id();

    match file.metadata_mut() {
        fs::FileContents::Binary(contents) => {
            if let Some(edit) = state.binary_backlog_mut().take(global_id) {
                contents.integrate_edit(edit, state.binary_ctx_mut());
            }
        },
        fs::FileContents::Symlink(_) => {},
        fs::FileContents::Text(contents) => {
            let local_id = state.local_id();
            for edit in state.text_backlog_mut().take(global_id) {
                contents.integrate_edit(edit, local_id);
            }
        },
    }
}

fn integrate_backlogged_annotations(
    file: fs::PuffFile<'_, Visible>,
    ctx: &mut text::TextCtx,
) {
    let local_id = file.local_id();
    let global_id = file.global_id();
    ctx.cursors.integrate_file_creation(local_id, global_id);
    ctx.selections.integrate_file_creation(local_id, global_id);
}

impl<'a> Create<'a> {
    /// TODO: docs.
    #[inline]
    pub fn node(&self) -> fs::Node<'_> {
        fs::Node::new(self.inner.node(), self.state)
    }
}

impl<'a> CreateAndResolve<'a> {
    /// TODO: docs.
    #[inline]
    pub fn create(&mut self) -> Create<'_> {
        Create { inner: self.inner.create(), state: self.state.as_ref() }
    }

    /// TODO: docs.
    #[inline]
    pub fn into_resolve(self) -> ResolveConflict<'a> {
        ResolveConflict { inner: self.inner.into_resolve(), state: self.state }
    }
}

impl<'a> Delete<'a> {
    /// TODO: docs.
    #[inline]
    pub fn node(&self) -> fs::Node<'_, Deleted> {
        fs::Node::new(self.inner.node(), self.state)
    }

    /// TODO: docs.
    #[inline]
    pub fn old_parent(&self) -> fs::Directory<'_> {
        fs::Directory::new(self.inner.old_parent(), self.state)
    }

    /// TODO: docs.
    #[inline]
    pub fn old_path(&self) -> AbsPathBuf {
        self.inner.old_path()
    }
}

impl<'a> Move<'a> {
    /// TODO: docs.
    #[inline]
    pub fn new_path(&self) -> AbsPathBuf {
        self.inner.new_path()
    }

    /// TODO: docs.
    #[inline]
    pub fn node(&self) -> fs::Node<'_> {
        fs::Node::new(self.inner.node(), self.state)
    }

    /// TODO: docs.
    #[inline]
    pub fn old_parent(&self) -> fs::Directory<'_> {
        fs::Directory::new(self.inner.old_parent(), self.state)
    }

    /// TODO: docs.
    #[inline]
    pub fn old_path(&self) -> AbsPathBuf {
        self.inner.old_path()
    }
}

impl<'a> MoveAndResolve<'a> {
    /// TODO: docs.
    #[inline]
    pub fn r#move(&mut self) -> Move<'_> {
        Move { inner: self.inner.r#move(), state: self.state.as_ref() }
    }

    /// TODO: docs.
    #[inline]
    pub fn into_resolve(self) -> ResolveConflict<'a> {
        ResolveConflict { inner: self.inner.into_resolve(), state: self.state }
    }
}

impl<'a> Rename<'a> {
    /// TODO: docs.
    #[inline]
    pub fn new_path(&self) -> AbsPathBuf {
        self.inner.new_path()
    }

    /// TODO: docs.
    #[inline]
    pub fn node(&self) -> fs::Node<'_> {
        fs::Node::new(self.inner.node(), self.state)
    }

    /// TODO: docs.
    #[inline]
    pub fn old_name(&self) -> &NodeName {
        self.inner.old_name()
    }

    /// TODO: docs.
    #[inline]
    pub fn old_path(&self) -> AbsPathBuf {
        self.inner.old_path()
    }

    /// TODO: docs.
    #[inline]
    pub fn parent(&self) -> fs::Directory<'_> {
        fs::Directory::new(self.inner.parent(), self.state)
    }
}

impl<'a> RenameAndResolve<'a> {
    /// TODO: docs.
    #[inline]
    pub fn rename(&mut self) -> Rename<'_> {
        Rename { inner: self.inner.rename(), state: self.state.as_ref() }
    }

    /// TODO: docs.
    #[inline]
    pub fn into_resolve(self) -> ResolveConflict<'a> {
        ResolveConflict { inner: self.inner.into_resolve(), state: self.state }
    }
}

impl<'a> ResolveConflict<'a> {
    /// TODO: docs.
    #[inline]
    pub fn assume_resolved(self) -> Result<(), Self> {
        self.inner
            .assume_resolved()
            .map_err(|inner| Self { inner, state: self.state })
    }

    /// TODO: docs.
    #[inline]
    pub fn conflicting_node(&self) -> fs::Node<'_> {
        fs::Node::new(self.inner.conflicting_node(), self.state.as_ref())
    }

    /// TODO: docs.
    #[inline]
    pub fn conflicting_node_mut(&mut self) -> fs::NodeMut<'_, Visible> {
        fs::NodeMut::new(
            self.inner.conflicting_node_mut(),
            self.state.reborrow(),
        )
    }

    /// TODO: docs.
    #[inline]
    pub fn existing_node(&self) -> fs::Node<'_> {
        fs::Node::new(self.inner.existing_node(), self.state.as_ref())
    }

    /// TODO: docs.
    #[inline]
    pub fn existing_node_mut(&mut self) -> fs::NodeMut<'_, Visible> {
        fs::NodeMut::new(self.inner.existing_node_mut(), self.state.reborrow())
    }
}
