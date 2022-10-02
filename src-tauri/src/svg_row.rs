use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use anyhow::{bail, Result};
use serde::{Serialize, Serializer};
use crate::parseable_info::ParseableCommitInfo;

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

#[derive(Clone)]
pub enum DrawProperty {
    SomeHashMap(HashMap<String, SVGProperty>),
    SomeVector(Vec<HashMap<String, SVGProperty>>),
    SomeVectorVector(Vec<Vec<HashMap<String, SVGProperty>>>),
}

impl Serialize for DrawProperty {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match &self {
            DrawProperty::SomeHashMap(hm) => hm.serialize(serializer),
            DrawProperty::SomeVector(v) => v.serialize(serializer),
            DrawProperty::SomeVectorVector(v) => v.serialize(serializer),
        }
    }
}

#[derive(Clone)]
pub enum RowProperty {
    SomeInt(isize),
    SomeString(String),
    SomeHashMap(HashMap<String, DrawProperty>),
}

impl Serialize for RowProperty {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match &self {
            RowProperty::SomeInt(i) => i.serialize(serializer),
            RowProperty::SomeString(s) => s.serialize(serializer),
            RowProperty::SomeHashMap(hm) => hm.serialize(serializer),
        }
    }
}

const Y_SPACING: isize = 24;  // If changing, be sure to update on front-end too
const Y_OFFSET: isize = 20;  // If changing, be sure to update on front-end too
const X_SPACING: isize = 15;
const X_OFFSET: isize = 20;
const TEXT_Y_OFFSET: isize = 5;
const CIRCLE_RADIUS: isize = 5;
const LINE_STROKE_WIDTH: isize = 2;
const RECT_HEIGHT: isize = 18;
const RECT_Y_OFFSET: isize = -(RECT_HEIGHT / 2);

#[derive(Clone)]
pub struct SVGRow {
    sha: String,
    summary: String,
    parent_oids: Vec<String>,
    child_oids: Vec<String>,
    has_parent_child_svg_rows_set: bool,
    parent_svg_rows: Vec<Rc<RefCell<SVGRow>>>,
    child_svg_rows: Vec<Rc<RefCell<SVGRow>>>,
    x: isize,
    y: isize,
}

impl SVGRow {
    pub fn from_commit_info(commit_info: &ParseableCommitInfo) -> Self {
        Self {
            sha: commit_info.borrow_sha().clone(),
            summary: commit_info.borrow_summary().clone(),
            parent_oids: commit_info.borrow_parent_shas().clone(),
            child_oids: commit_info.borrow_child_shas().clone(),
            has_parent_child_svg_rows_set: false,
            parent_svg_rows: vec![],
            child_svg_rows: vec![],
            x: commit_info.borrow_x().clone(),
            y: commit_info.borrow_y().clone(),
        }
    }

    pub fn set_parent_and_child_svg_row_values(&mut self, all_svg_rows: &HashMap<String, Rc<RefCell<SVGRow>>>) {
        for sha in &self.parent_oids {
            match all_svg_rows.get(&*sha) {
                Some(svg_row_rc) => {
                    self.parent_svg_rows.push(svg_row_rc.clone());
                },
                // If a parent is not present, ignore it. It may be outside the revwalk range.
                None => (),
            };
        }

        for sha in &self.child_oids {
            match all_svg_rows.get(&*sha) {
                Some(svg_row_rc) => {
                    self.child_svg_rows.push(svg_row_rc.clone());
                },
                // If a child is not present, ignore it. It may be outside the revwalk range.
                None => (),
            };
        }

        self.has_parent_child_svg_rows_set = true;
    }

    fn get_color_string(x: isize) -> String {
        let color_num = x % 4;
        if color_num == 0 {
            String::from("#00CC19")
        } else if color_num == 1 {
            String::from("#0198A6")
        } else if color_num == 2 {
            String::from("#FF7800")
        } else {
            String::from("#FF0D00")
        }
    }

    pub fn get_occupied_table(svg_rows: &mut Vec<Rc<RefCell<SVGRow>>>) -> Result<Vec<Vec<isize>>> {
        let mut main_table: Vec<Vec<isize>> = vec![];

        for svg_row_rc in svg_rows.iter_mut() {
            let mut svg_row = svg_row_rc.borrow_mut();

            if !svg_row.has_parent_child_svg_rows_set {
                bail!("SVGRow object didn't have parents or children set. Make sure 'set_parent_and_child_svg_row_values' is run before 'get_occupied_table'!");
            }

            // Set the current node position as occupied (or find a position that's unoccupied and occupy it).
            if svg_row.y < main_table.len() as isize {
                while main_table[svg_row.y as usize].contains(&svg_row.x) {
                    svg_row.x += 1;
                }
                main_table[svg_row.y as usize].push(svg_row.x);
            } else {
                main_table.push(vec![svg_row.x]);
            }

            // Set the space of the line from the current node to its parents as occupied.
            for parent_svg_row_rc in &svg_row.parent_svg_rows {
                let mut parent_svg_row = parent_svg_row_rc.borrow_mut();
                let mut moved_x_val = 0;
                for i in (svg_row.y + 1)..parent_svg_row.y {
                    let mut x_val = svg_row.x;
                    if i < main_table.len() as isize {
                        while main_table[i as usize].contains(&x_val) {
                            x_val += 1;
                            // Note: this has to stay in the loop so it's only set when x changes!
                            // and not just to svg_row.x
                            moved_x_val = x_val;
                        }
                        main_table[i as usize].push(x_val);
                    } else {
                        main_table.push(vec![x_val]);
                    }
                }
                // This is used particularly for merging lines
                parent_svg_row.x = moved_x_val;
            }
        }

        // Loop through after everything's set in order to properly occupy spaces by curved lines just for summary text positions.
        for svg_row_rc in svg_rows {
            let svg_row = svg_row_rc.borrow();

            for parent_svg_row_rc in &svg_row.parent_svg_rows {
                let parent_svg_row = parent_svg_row_rc.borrow();
                if svg_row.x < parent_svg_row.x {
                    let x_val = parent_svg_row.x;
                    main_table[svg_row.y as usize].push(x_val);
                } else if svg_row.x > parent_svg_row.x {
                    let x_val = svg_row.x;
                    main_table[parent_svg_row.y as usize].push(x_val);
                }
            }
        }

        Ok(main_table)
    }

    pub fn get_draw_properties(&mut self, main_table: &Vec<Vec<isize>>) -> HashMap<String, RowProperty> {
        let mut row_properties: HashMap<String, RowProperty> = HashMap::new();
        let mut draw_properties: HashMap<String, DrawProperty> = HashMap::new();

        row_properties.insert(String::from("sha"), RowProperty::SomeString(self.sha.clone()));

        let pixel_x = self.x * X_SPACING + X_OFFSET;
        let pixel_y = self.y * Y_SPACING + Y_OFFSET;
        row_properties.insert(String::from("pixel_y"), RowProperty::SomeInt(pixel_y));
        let color = SVGRow::get_color_string(self.x);
        let mut child_lines: Vec<HashMap<String, SVGProperty>> = vec![];
        // Draw the lines from the current node's children to itself.
        for child_svg_row_rc in &self.child_svg_rows {
            let child_svg_row = child_svg_row_rc.borrow();
            let child_pixel_x = child_svg_row.x * X_SPACING + X_OFFSET;
            let child_pixel_y = child_svg_row.y * Y_SPACING + Y_OFFSET;
            let before_y = self.y - 1;
            let before_pixel_y = before_y * Y_SPACING + Y_OFFSET;
            if before_pixel_y != child_pixel_y {
                let start_index;
                let end_index;
                let line_pixel_x;
                if self.x > child_svg_row.x {
                    line_pixel_x = pixel_x;
                    start_index = child_svg_row.y + 1;
                    end_index = before_y;
                } else {
                    line_pixel_x = child_pixel_x;
                    start_index = child_svg_row.y;
                    end_index = before_y - 1;
                }
                for i in start_index..=end_index {
                    let top_pixel_y = i * Y_SPACING + Y_OFFSET;
                    let bottom_pixel_y = (i + 1) * Y_SPACING + Y_OFFSET;

                    let mut style_str = String::from("stroke:");
                    style_str.push_str(&*SVGRow::get_color_string((line_pixel_x - X_OFFSET) / X_SPACING));
                    style_str.push_str(";stroke-width:");
                    style_str.push_str(&*LINE_STROKE_WIDTH.to_string());
                    let line_attrs: HashMap<String, SVGPropertyAttrs> = HashMap::from([
                        (String::from("x1"), SVGPropertyAttrs::SomeInt(line_pixel_x)),
                        (String::from("y1"), SVGPropertyAttrs::SomeInt(top_pixel_y)),
                        (String::from("x2"), SVGPropertyAttrs::SomeInt(line_pixel_x)),
                        (String::from("y2"), SVGPropertyAttrs::SomeInt(bottom_pixel_y)),
                        (String::from("style"), SVGPropertyAttrs::SomeString(style_str)),
                    ]);
                    child_lines.push(HashMap::from([
                        (String::from("tag"), SVGProperty::SomeString(String::from("line"))),
                        (String::from("attrs"), SVGProperty::SomeHashMap(line_attrs)),
                        (String::from("row-y"), SVGProperty::SomeInt(i + 1)),
                    ]));
                }
            }
            let mut style_str = String::from("stroke:");
            let row_y = self.y;
            if child_svg_row.x >= self.x {
                // Sets the color for "branching" lines and straight lines
                style_str.push_str(&*SVGRow::get_color_string(child_svg_row.x));
            } else {
                // Sets the color for "merging" lines
                style_str.push_str(&*SVGRow::get_color_string(self.x));
            }
            style_str.push_str(";fill:transparent;stroke-width:");
            style_str.push_str(&*LINE_STROKE_WIDTH.to_string());
            if child_pixel_x == pixel_x {
                let line_attrs: HashMap<String, SVGPropertyAttrs> = HashMap::from([
                    (String::from("x1"), SVGPropertyAttrs::SomeInt(child_pixel_x)),
                    (String::from("y1"), SVGPropertyAttrs::SomeInt(before_pixel_y)),
                    (String::from("x2"), SVGPropertyAttrs::SomeInt(pixel_x)),
                    (String::from("y2"), SVGPropertyAttrs::SomeInt(pixel_y)),
                    (String::from("style"), SVGPropertyAttrs::SomeString(style_str)),
                ]);
                child_lines.push(HashMap::from([
                    (String::from("tag"), SVGProperty::SomeString(String::from("line"))),
                    (String::from("attrs"), SVGProperty::SomeHashMap(line_attrs)),
                    (String::from("row-y"), SVGProperty::SomeInt(row_y)),
                ]));
            } else {
                let d_str;
                if child_pixel_x < pixel_x {
                    let after_child_pixel_y = (child_svg_row.y + 1) * Y_SPACING + Y_OFFSET;
                    let start_control_point_x = child_pixel_x + X_SPACING * 3 / 4;
                    let end_control_point_y = after_child_pixel_y - Y_SPACING * 3 / 4;
                    d_str = format!("M {child_pixel_x} {child_pixel_y} C {start_control_point_x} {child_pixel_y}, {pixel_x} {end_control_point_y}, {pixel_x} {after_child_pixel_y}");
                } else {
                    let start_control_point_y = before_pixel_y + Y_SPACING * 3 / 4;
                    let end_control_point_x = pixel_x + X_SPACING * 3 / 4;
                    d_str = format!("M {child_pixel_x} {before_pixel_y} C {child_pixel_x} {start_control_point_y}, {end_control_point_x} {pixel_y}, {pixel_x} {pixel_y}");
                }
                let path_attrs: HashMap<String, SVGPropertyAttrs> = HashMap::from([
                    (String::from("d"), SVGPropertyAttrs::SomeString(d_str)),
                    (String::from("style"), SVGPropertyAttrs::SomeString(style_str)),
                ]);
                child_lines.push(HashMap::from([
                    (String::from("tag"), SVGProperty::SomeString(String::from("path"))),
                    (String::from("attrs"), SVGProperty::SomeHashMap(path_attrs)),
                    (String::from("row-y"), SVGProperty::SomeInt(row_y)),
                ]));
            }
        }
        draw_properties.insert(String::from("child_lines"), DrawProperty::SomeVector(child_lines));

        // Now get the circle
        let circle_attrs: HashMap<String, SVGPropertyAttrs> = HashMap::from([
            (String::from("cx"), SVGPropertyAttrs::SomeInt(pixel_x)),
            (String::from("cy"), SVGPropertyAttrs::SomeInt(pixel_y)),
            (String::from("r"), SVGPropertyAttrs::SomeInt(CIRCLE_RADIUS)),
            (String::from("stroke"), SVGPropertyAttrs::SomeString(color.clone())),
            (String::from("stroke-width"), SVGPropertyAttrs::SomeInt(1)),
            (String::from("fill"), SVGPropertyAttrs::SomeString(color.clone())),
        ]);
        draw_properties.insert(String::from("circle"), DrawProperty::SomeHashMap(HashMap::from([
            (String::from("tag"), SVGProperty::SomeString(String::from("circle"))),
            (String::from("attrs"), SVGProperty::SomeHashMap(circle_attrs)),
        ])));

        let largest_occupied_x = main_table[self.y as usize].iter().max().unwrap_or(&0);

        // Get summary text
        let text_attrs: HashMap<String, SVGPropertyAttrs> = HashMap::from([
            (String::from("x"), SVGPropertyAttrs::SomeInt((largest_occupied_x + 1) * X_SPACING + X_OFFSET)),
            (String::from("y"), SVGPropertyAttrs::SomeInt(pixel_y + TEXT_Y_OFFSET)),
            (String::from("fill"), SVGPropertyAttrs::SomeString(String::from("white"))),
        ]);
        draw_properties.insert(String::from("summary_text"), DrawProperty::SomeHashMap(HashMap::from([
            (String::from("tag"), SVGProperty::SomeString(String::from("text"))),
            (String::from("attrs"), SVGProperty::SomeHashMap(text_attrs)),
            (String::from("textContent"), SVGProperty::SomeString(self.summary.clone())),
        ])));

        // Get background rectangle
        let rect_attrs: HashMap<String, SVGPropertyAttrs> = HashMap::from([
            (String::from("class"), SVGPropertyAttrs::SomeString(String::from("svg-hoverable-row"))),
            (String::from("x"), SVGPropertyAttrs::SomeInt(pixel_x)),
            (String::from("y"), SVGPropertyAttrs::SomeInt(pixel_y + RECT_Y_OFFSET)),
            (String::from("width"), SVGPropertyAttrs::SomeInt(0)),
            (String::from("height"), SVGPropertyAttrs::SomeInt(RECT_HEIGHT)),
            (String::from("style"), SVGPropertyAttrs::SomeString(String::from("fill:white;fill-opacity:0.1;"))),
        ]);
        draw_properties.insert(String::from("back_rect"), DrawProperty::SomeHashMap(HashMap::from([
            (String::from("tag"), SVGProperty::SomeString(String::from("rect"))),
            (String::from("attrs"), SVGProperty::SomeHashMap(rect_attrs)),
        ])));

        row_properties.insert(String::from("elements"), RowProperty::SomeHashMap(draw_properties));

        row_properties
    }

    pub fn get_branch_draw_properties(branches_and_tags: Vec<(String, String)>) -> Vec<Vec<HashMap<String, SVGProperty>>> {
        // Get the branch text
        let mut branch_and_tags: Vec<Vec<HashMap<String, SVGProperty>>> = vec![];
        for (branch_name, branch_type) in branches_and_tags.clone().into_iter() {
            let mut branch_and_tag_properties: Vec<HashMap<String, SVGProperty>> = vec![];
            let text_attrs: HashMap<String, SVGPropertyAttrs> = HashMap::from([
                (String::from("x"), SVGPropertyAttrs::SomeInt(0)),
                (String::from("y"), SVGPropertyAttrs::SomeInt(TEXT_Y_OFFSET)),
                (String::from("fill"), SVGPropertyAttrs::SomeString(String::from("white"))),
            ]);
            branch_and_tag_properties.push(HashMap::from([
                (String::from("tag"), SVGProperty::SomeString(String::from("text"))),
                (String::from("attrs"), SVGProperty::SomeHashMap(text_attrs)),
                (String::from("textContent"), SVGProperty::SomeString(branch_name.clone())),
            ]));

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
}
