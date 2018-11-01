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
            Pattern::element("envar"),
            Pattern::element("uri"),
            Pattern::element("command"),
            Pattern::element("link"),
            Pattern::element("xref"),
            Pattern::element("replaceable"),
        ]);

        let block = Pattern::Choice(vec![
            Pattern::para(Pattern::many1(inline.clone())),
            Pattern::element("dinkus"),
            Pattern::element("listing"),
            Pattern::element("screen"),
            Pattern::element("procedure"),
        ]);

        let title = Pattern::many1(inline.clone());

        let uri_string = Pattern::Text;

        // FIXME: should be a distinct type for validation.
        let id_string = Pattern::Text;

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
                    Pattern::many(Pattern::element("simplesect")),
                    Pattern::many(Pattern::element("subsection"))
                ])
            ]
        );

        schema.add_element(
            "subsection",
            vec![
                title.clone(),
                Pattern::Seq(vec![
                    Pattern::many(block.clone()),
                    Pattern::many(Pattern::element("simplesect")),
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

        for tag in ["emph", "remark", "code", "filename", "envar", "uri", "command", "replaceable"].iter() {
            schema.add_element(
                tag,
                vec![
                    Pattern::many(inline.clone())
                ]
            );
        }

        schema.add_element(
            "link",
            vec![
                uri_string.clone(),
                Pattern::many(inline.clone())
            ]
        );

        schema.add_element(
            "xref",
            vec![
                id_string.clone()
            ]
        );

        schema.add_element("dinkus", vec![]);

        schema.add_element(
            "listing",
            vec![
                Pattern::many(
                    Pattern::Text
                )
            ]
        );

        schema.add_element(
            "screen",
            vec![
                Pattern::many(
                    Pattern::Choice(vec![
                        Pattern::Text,
                        Pattern::element("replaceable"),
                    ])
                )
            ]
        );

        schema.add_element(
            "procedure",
            vec![
                Pattern::many(Pattern::element("step")),
            ]
        );

        schema.add_element(
            "step",
            vec![
                Pattern::many(block.clone()),
            ]
        );

        schema
    };
}
