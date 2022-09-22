use std::collections::HashMap;
use anyhow::Result;
use serde::{Serialize, Serializer};
use crate::parseable_info::BranchNameAndType;

#[derive(Clone)]
pub enum SVGPropertyAttrs {
    SomeString(String),
    SomeInt(isize),
}

impl Serialize for SVGPropertyAttrs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match &self {
            SVGPropertyAttrs::SomeString(st) => st.serialize(serializer),
            SVGPropertyAttrs::SomeInt(i) => i.serialize(serializer),
        }
    }
}

#[derive(Clone)]
pub enum SVGProperty {
    SomeInt(isize),
    SomeString(String),
    SomeHashMap(HashMap<String, SVGPropertyAttrs>),
}

impl Serialize for SVGProperty {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match &self {
            SVGProperty::SomeInt(i) => i.serialize(serializer),
            SVGProperty::SomeString(st) => st.serialize(serializer),
            SVGProperty::SomeHashMap(hm) => hm.serialize(serializer),
        }
    }
}

const TEXT_Y_OFFSET: isize = 5;
const RECT_HEIGHT: isize = 18;  // If changing, be sure to update on the front-end as well!
const RECT_Y_OFFSET: isize = -(RECT_HEIGHT / 2);

// TODO: Move this to the front-end maybe?
pub fn get_branch_draw_properties(branches_and_tags: Vec<BranchNameAndType>) -> Vec<Vec<HashMap<String, SVGProperty>>> {
    // Get the branch text
    let mut branch_and_tags: Vec<Vec<HashMap<String, SVGProperty>>> = vec![];
    for branch_name_and_type in branches_and_tags {
        let mut branch_and_tag_properties: Vec<HashMap<String, SVGProperty>> = vec![];
        let text_attrs: HashMap<String, SVGPropertyAttrs> = HashMap::from([
            (String::from("x"), SVGPropertyAttrs::SomeInt(0)),
            (String::from("y"), SVGPropertyAttrs::SomeInt(TEXT_Y_OFFSET)),
            (String::from("fill"), SVGPropertyAttrs::SomeString(String::from("white"))),
        ]);
        branch_and_tag_properties.push(HashMap::from([
            (String::from("tag"), SVGProperty::SomeString(String::from("text"))),
            (String::from("attrs"), SVGProperty::SomeHashMap(text_attrs)),
            (String::from("textContent"), SVGProperty::SomeString(branch_name_and_type.borrow_shorthand().clone())),
        ]));

        let branch_type = branch_name_and_type.borrow_branch_type();
        let mut branch_rect_color = "yellow";
        if branch_type == "local" {
            branch_rect_color = "red";
        } else if branch_type == "remote" {
            branch_rect_color = "green";
        } else if branch_type == "tag" {
            branch_rect_color = "grey";
        }

        let mut style_str = String::from("fill:");
        style_str.push_str(branch_rect_color);
        style_str.push_str(";fill-opacity:0.5;");
        let rect_attrs: HashMap<String, SVGPropertyAttrs> = HashMap::from([
            (String::from("x"), SVGPropertyAttrs::SomeInt(0)),
            (String::from("y"), SVGPropertyAttrs::SomeInt(RECT_Y_OFFSET)),
            (String::from("rx"), SVGPropertyAttrs::SomeInt(10)),
            (String::from("ry"), SVGPropertyAttrs::SomeInt(10)),
            (String::from("width"), SVGPropertyAttrs::SomeInt(0)),
            (String::from("height"), SVGPropertyAttrs::SomeInt(RECT_HEIGHT)),
            (String::from("style"), SVGPropertyAttrs::SomeString(style_str)),
        ]);
        branch_and_tag_properties.push(HashMap::from([
            (String::from("tag"), SVGProperty::SomeString(String::from("rect"))),
            (String::from("attrs"), SVGProperty::SomeHashMap(rect_attrs)),
        ]));
        branch_and_tags.push(branch_and_tag_properties);
    }
    branch_and_tags
}
