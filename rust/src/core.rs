use schema::*;
use std::collections::HashMap;

lazy_static! {
    pub static ref SCHEMA: Schema = {
        let mut schema = Schema {
            start: Pattern::Choice(vec![
                Pattern::element("book"),
                Pattern::element("chapter"),
            ]),
            elements: HashMap::new()
        };

        let inline = Pattern::Choice(vec![
            Pattern::Text,
            Pattern::element("emph"),
            Pattern::element("code"),
            Pattern::element("remark"),
        ]);

        let block = Pattern::Choice(vec![
            Pattern::para(Pattern::many1(inline.clone())),
            Pattern::element("stars"),
        ]);

        schema.add_element(
            "book",
            vec![
                Pattern::many1(inline.clone()),
                Pattern::many(Pattern::element("chapter"))
            ]
        );

        schema.add_element(
            "chapter",
            vec![
                Pattern::many1(inline.clone()),
                Pattern::Seq(vec![
                    Pattern::many(block.clone()),
                    Pattern::many(Pattern::element("section"))
                ])
            ]
        );

        schema.add_element(
            "section",
            vec![
                Pattern::many1(inline.clone()),
                Pattern::Seq(vec![
                    Pattern::many(block.clone()),
                ])
            ]
        );

        schema.add_element(
            "emph",
            vec![
                Pattern::many(inline.clone())
            ]
        );

        schema.add_element(
            "remark",
            vec![
                Pattern::many(inline.clone())
            ]
        );

        schema.add_element(
            "code",
            vec![
                Pattern::many(inline.clone())
            ]
        );

        schema.add_element("stars", vec![]);

        schema
    };
}
