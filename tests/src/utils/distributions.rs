use rand::Rng;
use rand::distr::Distribution;

/// A [`Distribution`] that samples `char`s in the `a..z` and `A..Z` range
/// with uniform probability.
pub(crate) struct AsciiLetterDistribution;

/// A [`Distribution`] that samples `char`s to roughly match the average line
/// and word lengths of source code, with a 10% bias towards emojis.
pub(crate) struct CodeDistribution;

/// A [`Distribution`] that samples `char`s from a set of emoji characters
/// with uniform probability.
pub(crate) struct EmojiDistribution;

impl CodeDistribution {
    const AVG_LINE_LEN: usize = 80;
    const AVG_WORD_LEN: usize = 5;
    const EMOJI_PROBABILITY: f32 = 0.1;
}

impl EmojiDistribution {
    const SAMPLE_SPACE: &[char] = &[
        '😀', '😃', '😄', '😁', '😆', '😅', '🤣', '😂', '🙂', '🙃', '😉',
        '😊', '😇', '🥰', '😍', '🤩', '😘', '😗', '😚', '😋', '😛', '😝',
        '😜', '🤪', '🤨', '🧐', '🤓', '😎', '🤩', '🥳', '😏', '😒', '😞',
        '😔', '😟', '😕', '🙁', '😣', '😖', '😫', '😩', '🥺', '😢', '😭',
        '😤', '😠', '😡', '🤬', '🤯', '😳', '🥵', '🥶', '😱', '😨', '😰',
        '😥', '😓', '🤗', '🤔', '🤭', '🤫', '🤥', '😶', '😐', '😑', '😬',
        '🙄', '😯', '😦', '😧', '😮', '😲', '🥱', '😴', '🤤', '😪', '😵',
        '🤐', '🥴', '🤢', '🤮', '🤧', '😷', '🤒', '🤕', '🤑', '🤠', '😈',
        '👿', '👹', '👺', '🤡', '💩', '👻', '💀', '🦆', '🦀', '🐎', '🦖',
        '🐤',
    ];
}

impl Distribution<char> for AsciiLetterDistribution {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> char {
        if rng.random_bool(0.5) {
            rng.random_range('a'..='z')
        } else {
            rng.random_range('A'..='Z')
        }
    }
}

impl Distribution<char> for CodeDistribution {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> char {
        let emoji_range = 0f32..Self::EMOJI_PROBABILITY;

        let newline_range = emoji_range.end
            ..(emoji_range.end + 1f32 / (Self::AVG_LINE_LEN as f32));

        let space_range = newline_range.end
            ..(newline_range.end + 1f32 / (Self::AVG_WORD_LEN as f32));

        match rng.random_range(0f32..=1f32) {
            x if emoji_range.contains(&x) => EmojiDistribution.sample(rng),
            x if newline_range.contains(&x) => '\n',
            x if space_range.contains(&x) => ' ',
            _ => AsciiLetterDistribution.sample(rng),
        }
    }
}

impl Distribution<char> for EmojiDistribution {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> char {
        Self::SAMPLE_SPACE[rng.random_range(0..Self::SAMPLE_SPACE.len())]
    }
}
