use editor::{AccessMut, AgentId, BorrowState, Buffer, Context, Editor};

pub(crate) trait TestEditor: Editor {
    fn create_scratch_buffer(
        this: impl AccessMut<Self>,
        agent_id: AgentId,
    ) -> impl Future<Output = Self::BufferId>;
}

pub(crate) trait ContextExt<Ed: TestEditor> {
    fn create_scratch_buffer(
        &mut self,
        agent_id: AgentId,
    ) -> impl Future<Output = Ed::BufferId>;

    fn create_and_focus_scratch_buffer(
        &mut self,
        agent_id: AgentId,
    ) -> impl Future<Output = Ed::BufferId>;
}

#[cfg(feature = "neovim")]
impl TestEditor for neovim::Neovim {
    async fn create_scratch_buffer(
        mut this: impl AccessMut<Self>,
        _: AgentId,
    ) -> Self::BufferId {
        use neovim::oxi::api::{self, opts};
        use neovim::tests::NeovimExt;

        let buf_id = this.create_scratch_buffer();

        // The (fix)eol options mess us the fuzzy edits tests because inserting
        // text when the buffer is empty will also cause a trailing \n to be
        // inserted, so unset them.
        let opts = opts::OptionOpts::builder().buffer(buf_id.into()).build();
        api::set_option_value::<bool>("eol", false, &opts).unwrap();
        api::set_option_value::<bool>("fixeol", false, &opts).unwrap();

        buf_id
    }
}

impl TestEditor for mock::Mock {
    async fn create_scratch_buffer(
        mut this: impl AccessMut<Self>,
        agent_id: AgentId,
    ) -> Self::BufferId {
        let scratch_file_path = |num_scratch: u32| {
            let file_name = format!("scratch-{num_scratch}")
                .parse::<abs_path::NodeNameBuf>()
                .expect("it's valid");
            abs_path::AbsPathBuf::root().join(&file_name)
        };

        let mut num_scratch = 0;

        loop {
            let file_path = scratch_file_path(num_scratch);
            if this.with_mut(|mock| mock.buffer_at_path(&file_path).is_none())
            {
                return Self::create_buffer(this, &file_path, agent_id)
                    .await
                    .expect("couldn't create buffer");
            }
            num_scratch += 1;
        }
    }
}

impl<Ed: TestEditor, Bs: BorrowState> ContextExt<Ed> for Context<Ed, Bs>
where
    Self: AccessMut<Ed>,
{
    async fn create_scratch_buffer(
        &mut self,
        agent_id: AgentId,
    ) -> Ed::BufferId {
        Ed::create_scratch_buffer(self, agent_id).await
    }

    async fn create_and_focus_scratch_buffer(
        &mut self,
        agent_id: AgentId,
    ) -> Ed::BufferId {
        let buffer_id = self.create_scratch_buffer(agent_id).await;
        self.with_editor(|ed| {
            if let Some(mut buffer) = ed.buffer(buffer_id.clone()) {
                buffer.schedule_focus(agent_id);
            }
        });
        buffer_id
    }
}
