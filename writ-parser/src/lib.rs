use chumsky::Parser;
use chumsky::prelude::{end, Input};

pub mod cst;


fn parser<'src, I: Input<'src>>() -> impl Parser<'src, I, ()> {
    end() // --(5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // Our parser expects empty strings, so this should parse successfully
        assert_eq!(parser().parse("").into_result(), Ok(()));

        // Anything other than an empty string should produce an error
        assert!(parser().parse("123").has_errors());
    }
}
