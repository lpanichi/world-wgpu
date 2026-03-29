// Maybe use num-trait to have a generic method on floats ?
pub fn linspace(start: f32, end: f32, n_steps: usize) -> Vec<f32> {
    let barrier_length = (end - start) / (n_steps - 1) as f32;
    let mut poles = vec![0.0; n_steps];
    for i in 0..n_steps {
        poles[i] = i as f32 * barrier_length + start;
    }
    poles
}

#[cfg(test)]
mod commons_test {
    use super::*;
    use assertor::*;

    #[test]
    fn split_range() {
        /* Setup */
        let start = 2.3;
        let end = 6.8;
        let n_steps = 7;

        /* Test */
        let result = linspace(start, end, n_steps);
        assert_that!(result[0]).is_equal_to(2.3);
        assert_that!(result[1]).is_equal_to(3.05);
        assert_that!(result[2]).is_equal_to(3.8);
        assert_that!(result[3]).is_equal_to(4.55);
        assert_that!(result[4]).is_equal_to(5.3);
        assert_that!(result[5]).is_equal_to(6.05);
        assert_that!(result[6]).is_equal_to(6.8);
    }
}
