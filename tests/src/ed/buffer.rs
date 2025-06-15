use core::{fmt, iter, mem};

use ed::{Buffer, Context, Edit, Editor, Replacement};
use futures_util::stream::{FusedStream, StreamExt};
use rand::Rng;

use crate::ed::{ContextExt, TestEditor};
use crate::utils::{CodeDistribution, fuzz};

pub(crate) async fn fuzz_edits(
    num_epochs: u32,
    ctx: &mut Context<impl TestEditor>,
) {
    let agent_id = ctx.new_agent_id();

    let buf_id = ctx.create_scratch_buffer(agent_id).await;

    let mut edits = Edit::new_stream(buf_id.clone(), ctx);

    // A string to which we'll apply the same edits we apply to the buffer.
    let mut expected_contents = ctx.with_borrowed(|ctx| {
        let buf = ctx.buffer(buf_id.clone()).unwrap();
        buf.get_text(0..buf.byte_len()).to_string()
    });

    fuzz::run_async(async |rng| {
        for epoch_idx in 0..num_epochs {
            let replacement = random_replacement(&expected_contents, rng);

            // Apply the replacement to the string.
            expected_contents.replace_range(
                replacement.removed_range(),
                replacement.inserted_text(),
            );

            // Apply the replacement to the buffer.
            ctx.with_borrowed(|ctx| {
                let mut buf = ctx.buffer(buf_id.clone()).unwrap();
                buf.edit(iter::once(replacement.clone()), agent_id);
            });

            // Wait to be notified about the edit we just made.
            let edit = edits.next().await.unwrap();

            // Make sure the edit we got notified about matches the one we
            // applied.
            assert_eq!(edit.made_by, agent_id);
            assert_eq!(&*edit.replacements, &[replacement]);

            // Make sure the buffer's contents are the same as the string.
            ctx.with_borrowed(|ctx| {
                let buf = ctx.buffer(buf_id.clone()).unwrap();
                let buf_contents = buf.get_text(0..buf.byte_len());

                if buf_contents != &*expected_contents {
                    panic!(
                        "buffer and string diverged after {num_epochs} \
                         epochs:\n{lhs}\nvs\n{rhs}",
                        num_epochs = epoch_idx + 1,
                        lhs = DisplayBufContents(buf_contents),
                        rhs = DisplayBufContents(&expected_contents),
                    );
                }
            });
        }
    })
    .await;
}

/// Generates a random [`Replacement`] to be applied to the given string.
///
/// The replacement is guaranteed to not be a no-op (i.e. it either deletes
/// characters, inserts some, or both), and the [`ByteOffset`](ed::ByteOffset)s
/// representing the range to delete are guaranteed to be valid char boundaries
/// in the string.
fn random_replacement(s: &str, rng: &mut impl Rng) -> Replacement {
    // Taken from u8::is_utf8_char_boundary(), which is not public.
    let is_char_boundary = |byte: u8| (byte as i8) >= -0x40;

    let clip_to_char_boundary = |offset| {
        if offset >= s.len() {
            return s.len();
        }

        let num_bytes_to_prev_boundary = s.as_bytes()[..offset + 1]
            .iter()
            .rev()
            .position(|&byte| is_char_boundary(byte))
            .unwrap();

        let num_bytes_to_next_boundary = s.as_bytes()[offset..]
            .iter()
            .position(|&byte| is_char_boundary(byte))
            .unwrap_or(s.len() - offset);

        if num_bytes_to_prev_boundary <= num_bytes_to_next_boundary {
            offset - num_bytes_to_prev_boundary
        } else {
            offset + num_bytes_to_next_boundary
        }
    };

    // Make the average number of inserted bytes greater than the average
    // number of deleted bytes to let the buffer grow over time.
    let delete_num = rng.random_range(0..3);
    let insert_num = rng.random_range(0..5);

    let delete_from = clip_to_char_boundary(rng.random_range(0..=s.len()));
    let delete_to = clip_to_char_boundary(delete_from + delete_num);

    // If we generated a no-op replacement, try again.
    if (delete_from..delete_to).len() + insert_num == 0 {
        return random_replacement(s, rng);
    }

    let insert_str = iter::repeat_with(|| rng.sample(CodeDistribution))
        .take(insert_num)
        .collect::<String>();

    Replacement::new(delete_from..delete_to, insert_str)
}

pub(crate) trait EditExt {
    /// Returns a never-ending stream of [`Edit`]s on the buffer with the given
    /// ID.
    fn new_stream<Ed: Editor>(
        buf_id: Ed::BufferId,
        ctx: &mut Context<Ed>,
    ) -> impl FusedStream<Item = Edit> + Unpin + use<Self, Ed> {
        use ed::Buffer;

        let (tx, rx) = flume::unbounded();

        ctx.with_borrowed(|ctx| {
            mem::forget(ctx.buffer(buf_id).unwrap().on_edited(
                move |_buf, edit| {
                    let _ = tx.send(edit.clone());
                },
            ));
        });

        rx.into_stream()
    }
}

impl EditExt for Edit {}

struct DisplayBufContents<T>(T);

impl<T: fmt::Display> fmt::Display for DisplayBufContents<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "```\n{}\n```", self.0)
    }
}
