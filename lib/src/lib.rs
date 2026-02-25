/*
 * SPDX-FileCopyrightText: 2025 SyoBoN <syobon@syobon.net>
 *
 * SPDX-License-Identifier: UPL-1.0
 */

pub mod markov;
pub mod notestock;

#[cfg(test)]
mod test {
    use super::*;

    // 自前で用意
    const ZIP: &[u8] = include_bytes!("test.zip");

    #[test]
    fn test() {
        let data = notestock::parse(ZIP).unwrap();
        let mut builder = markov::MarkovBuilder::new();
        builder.learn_many(&data);
        let model = builder.build();
        let mut generator = markov::MarkovGenerator::from_model(model);
        generator.generate().unwrap();
    }
}
