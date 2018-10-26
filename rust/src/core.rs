use schema::*;
use std::collections::HashMap;

lazy_static! {
    pub static ref SCHEMA: Schema = {
        let mut schema = Schema {
            start: Pattern::Choice(vec![
                Pattern::element("book"),
                Pattern::element("part"),
                Pattern::element("chapter"),
            ]),
            elements: HashMap::new()
        };

        let inline = Pattern::Choice(vec![
            Pattern::Text,
            Pattern::element("emph"),
            Pattern::element("code"),
            Pattern::element("remark"),
            Pattern::element("filename"),
        ]);

        let block = Pattern::Choice(vec![
            Pattern::para(Pattern::many1(inline.clone())),
            Pattern::element("dinkus"),
            Pattern::element("listing"),
        ]);

        let title = Pattern::many1(inline.clone());

        schema.add_element(
            "book",
            vec![
                title.clone(),
                Pattern::many(Pattern::element("chapter"))
            ]
        );

        schema.add_element(
            "part",
            vec![
                title.clone(),
                Pattern::many(Pattern::element("chapter"))
            ]
        );

        schema.add_element(
            "chapter",
            vec![
                title.clone(),
                Pattern::Seq(vec![
                    Pattern::many(block.clone()),
                    Pattern::many(Pattern::element("simplesect")),
                    Pattern::many(Pattern::element("section"))
                ])
            ]
        );

        schema.add_element(
            "section",
            vec![
                title.clone(),
                Pattern::Seq(vec![
                    Pattern::many(block.clone()),
                ])
            ]
        );

        schema.add_element(
            "simplesect",
            vec![
                title.clone(),
                Pattern::Seq(vec![
                    Pattern::many(block.clone()),
                ])
            ]
        );

        for tag in ["emph", "remark", "code", "filename"].iter() {
            schema.add_element(
                tag,
                vec![
                    Pattern::many(inline.clone())
                ]
            );
        }

        schema.add_element("dinkus", vec![]);

        schema.add_element(
            "listing",
            vec![
                Pattern::Text
            ]
        );

        schema
    };
}
