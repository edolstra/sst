use schema::*;
use std::collections::HashMap;

lazy_static! {
    pub static ref SCHEMA: Schema = {
        let mut schema = Schema {
            start: Pattern::element("chapter"),
            elements: HashMap::new()
        };

        let inline = Pattern::Choice(vec![
            Pattern::Text,
            Pattern::element("emph"),
            Pattern::element("code"),
            Pattern::element("remark"),
        ]);

        let block = Pattern::Choice(vec![
            Pattern::para(Pattern::many(inline.clone()))
        ]);

        schema.add_element(
            Element::new(
                "chapter",
                vec![
                    Pattern::many1(inline.clone()),
                    Pattern::Seq(vec![
                        Pattern::many(block.clone()),
                        Pattern::many(Pattern::element("section"))
                    ])
                ]
            ));

        schema.add_element(
            Element::new(
                "emph",
                vec![
                    Pattern::many(inline.clone())
                ]
            ));

        schema
    };
}
