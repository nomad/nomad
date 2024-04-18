use nomad::test::{Generator, TestResult};
use nomad::{NvimBuffer, NvimEdit, Shared};

#[nomad::test]
fn nomad_buffer_sync_fuzz_0(gen: &mut Generator) -> TestResult {
    buffer_sync(8, gen)
}

#[nomad::test]
fn nomad_buffer_sync_fuzz_1(gen: &mut Generator) -> TestResult {
    buffer_sync(32, gen)
}

#[nomad::test]
fn nomad_buffer_sync_fuzz_2(gen: &mut Generator) -> TestResult {
    buffer_sync(256, gen)
}

#[nomad::test]
fn nomad_buffer_sync_fuzz_3(gen: &mut Generator) -> TestResult {
    buffer_sync(1024, gen)
}

/// Tests that a `NvimBuffer` stays synced with a string after a series of
/// edits.
fn buffer_sync(num_edits: usize, gen: &mut Generator) -> TestResult {
    let mut buffer = NvimBuffer::create();

    let string = Shared::new(String::new());

    {
        let mut string = string.clone();

        buffer.on_edit(move |edit| {
            let range = edit.start().into()..edit.end().into();
            string.replace(range, edit.replacement());
        });
    }

    for _ in 0..num_edits {
        let start = string.with(|s| gen.generate(s));
        let end = string.with(|s| gen.generate(&s[start..])) + start;
        let replacement = gen.generate::<String>(5);
        let edit = NvimEdit::new(start, end, replacement);
        buffer.edit(edit);
    }

    string.with(|s| {
        assert_eq!(buffer.get(..), s);
    });
}
