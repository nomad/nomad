use core::ops::Range;

use crop::Rope;
use nomad::test::{Generator, MeanLen, ReplacementCtx};
use nomad::{ByteOffset, IntoCtx, NvimBuffer, Point, Replacement, Shared};

#[nomad::test]
fn nomad_buffer_sync_fuzz_0(gen: &mut Generator) {
    buffer_sync(8, gen)
}

#[nomad::test]
fn nomad_buffer_sync_fuzz_1(gen: &mut Generator) {
    buffer_sync(32, gen)
}

#[nomad::test]
fn nomad_buffer_sync_fuzz_2(gen: &mut Generator) {
    buffer_sync(256, gen)
}

#[nomad::test]
fn nomad_buffer_sync_fuzz_3(gen: &mut Generator) {
    buffer_sync(1024, gen)
}

/// Tests that a `NvimBuffer` stays synced with a string after a series of
/// edits.
fn buffer_sync(num_edits: usize, gen: &mut Generator) {
    let mut buffer = NvimBuffer::create();

    let rope = Shared::new(Rope::new());

    {
        let rope = rope.clone();

        buffer.on_edit(move |edit| {
            let range: Range<usize> = edit.start().into()..edit.end().into();
            rope.with_mut(|r| r.replace(range, edit.replacement()));
        });
    }

    for _ in 0..num_edits {
        let replacement = rope.with(|r| {
            let ctx = ReplacementCtx::new(r.as_ref(), MeanLen(3), MeanLen(5));
            let rep: Replacement<ByteOffset> = gen.generate(ctx);
            let point_range = rep.range().into_ctx(r);
            Replacement::<Point<_>>::new(point_range, rep.replacement())
        });

        buffer.edit(replacement);
    }

    rope.with(|r| {
        assert_eq!(&buffer.get(..).unwrap(), r);
    });
}
