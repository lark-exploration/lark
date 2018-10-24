mod common;

#[cfg(test)]
mod tests {
    use lark_eval::{eval_context, IOHandler};

    #[test]
    fn eval_simple_add_test() {
        let (context, starting_fn_id) = crate::common::generate_simple_add_test();

        let mut eval_output = IOHandler::new(true);
        eval_context(&context, starting_fn_id, &mut eval_output);

        assert_eq!(Some("18\n".to_string()), eval_output.redirect);
    }

    #[test]
    fn eval_big_test() {
        let (context, starting_fn_id) = crate::common::generate_big_test();

        let mut eval_output = IOHandler::new(true);
        eval_context(&context, starting_fn_id, &mut eval_output);

        assert_eq!(Some("3\n18\n".to_string()), eval_output.redirect);
    }
}
