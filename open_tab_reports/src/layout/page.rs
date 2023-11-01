use std::{collections::HashMap, marker::PhantomData, hash::Hash};

use itertools::Itertools;
use serde::{Deserialize, Serialize, ser::{SerializeStruct, SerializeMap}, de::Visitor};

use super::{layout_trait::{Layoutable, Layouter}, LayoutContext, LayoutValue, LayoutedElements, DynamicTextBox, FixedImage, LayoutedDocument, FormName};


#[derive(Serialize, Deserialize)]
pub struct DocumentTemplate {
    pub forms: HashMap<String, SinglePageTemplate>,
    pub page_generators: Vec<Box<dyn PageLayoutable>>
}

impl DocumentTemplate {
    pub fn new(forms: HashMap<String, SinglePageTemplate>, page_generators: Vec<Box<dyn PageLayoutable>>) -> Self {
        Self {
            forms,
            page_generators
        }
    }

    pub fn layout(&self, context: &mut LayoutContext, value: &LayoutValue) -> LayoutedDocument {
        let dict_val = match value {
            LayoutValue::Dict(d) => d,
            _ => panic!("Expected Vec")
        };

        let empty = HashMap::new();
        let form_dict = match dict_val.get("forms") {
            Some(LayoutValue::Dict(d)) => d,
            None => &empty,
            _ => panic!("Expected dict")
        };

        let vec_val = match dict_val.get("pages") {
            Some(LayoutValue::Vec(v)) => v,
            _ => panic!("Expected Vec")
        };

        for (form_name, form) in self.forms.iter() {
            let elements = form.layout(context, form_dict.get(form_name).unwrap_or(&LayoutValue::None));
            context.graphics_collection.register_form(FormName(form_name.clone()), elements.elements);
        }

        let mut layouted_elements = vec![];
        for (page_generator, val) in self.page_generators.iter().zip(vec_val.iter()) {
            layouted_elements.extend(page_generator.layout_pages(context, val));
        }

        LayoutedDocument {
            pages: layouted_elements
        }
    }
}


#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum Format {
    A4Vertical,
    A4Horizontal,
}

pub struct PageLayoutInfo {
    pub format: Format,
    pub elements: LayoutedElements
}

impl Format {
    pub fn get_dimensions(&self) -> (f32, f32) {
        match self {
            Format::A4Vertical => (595.0, 842.0),
            Format::A4Horizontal => (842.0, 595.0)
        }
    }
}

#[typetag::serde(tag = "type")]
pub trait PageLayoutable {
    fn layout_pages(&self, context: &mut LayoutContext, value: &LayoutValue) -> Vec<PageLayoutInfo>;
}

#[derive(Serialize, Deserialize)]
pub struct TemplateEntry {
    pub key: Option<String>,
    pub element: Box<dyn Layoutable>
}

fn default_format() -> Format {
    Format::A4Vertical
}

#[derive(Serialize, Deserialize)]
pub struct SinglePageTemplate {
    pub elements: Vec<TemplateEntry>,
    #[serde(default = "default_format")]
    pub format: Format
}


impl SinglePageTemplate {
    pub fn new(format: Format, elements: Vec<TemplateEntry>) -> Self {
        Self {
            format,
            elements
        }
    }

    pub fn layout(&self, context: &mut LayoutContext, value: &LayoutValue) -> PageLayoutInfo {
        let mut layouted_elements = LayoutedElements::new();

        for entry in self.elements.iter() {
            let element_value = match (&entry.key, value) {
                (Some(key), LayoutValue::Dict(dict)) => dict.get(key).unwrap_or(&LayoutValue::None),
                _ => &LayoutValue::None
            };
            
            let layout_result = entry.element.layout(context, element_value);

            layouted_elements.extend(layout_result.objects.into_iter());
        }

        PageLayoutInfo {
            format: self.format.clone(),
            elements: layouted_elements,
        }
    }

    pub fn layout_many(&self, context: &mut LayoutContext, values: &LayoutValue) -> Vec<PageLayoutInfo> {
        let mut layouted_elements = vec![];

        let values = match values {
            LayoutValue::Vec(vec) => vec,
            _ => panic!("Expected Vec")
        };

        for value in values.iter() {
            layouted_elements.push(self.layout(context, value));
        }

        layouted_elements
    }
}

#[typetag::serde]
impl PageLayoutable for SinglePageTemplate {
    fn layout_pages(&self, context: &mut LayoutContext, value: &LayoutValue) -> Vec<PageLayoutInfo> {
        self.layout_many(context, value)
    }
}


#[derive(Serialize, Deserialize)]
pub struct ExpandingPageTemplate {
    pub elements: Vec<TemplateEntry>,
    #[serde(default = "default_format")]
    pub format: Format
}


impl ExpandingPageTemplate {
    pub fn new(format: Format, elements: Vec<TemplateEntry>) -> Self {
        Self {
            format,
            elements
        }
    }

    pub fn layout(&self, context: &mut LayoutContext, value: &LayoutValue) -> Vec<PageLayoutInfo> {
        dbg!(&value);

        let mut pages = vec![];

        for entry in self.elements.iter() {
            let element_value = match (&entry.key, value) {
                (Some(key), LayoutValue::Dict(dict)) => dict.get(key).unwrap_or(&LayoutValue::None),
                _ => &LayoutValue::None
            };

            let mut layouter = entry.element.get_layouter(element_value);

            let (max_width, max_height) = self.format.get_dimensions();

            loop {
                let mut layouted_elements = LayoutedElements::new();

                let layout_result = layouter.layout(context, super::layout_trait::BoundingBox {
                    x: 0.0,
                    y: 0.0,
                    width: max_width,
                    height: max_height
                });

                layouted_elements.extend(layout_result.objects.into_iter());
                pages.push(PageLayoutInfo {
                    format: self.format.clone(),
                    elements: layouted_elements,
                });

                if layout_result.is_done {
                    break;
                }
            }
        }

        pages
    }
}

#[typetag::serde]
impl PageLayoutable for ExpandingPageTemplate {
    fn layout_pages(&self, context: &mut LayoutContext, value: &LayoutValue) -> Vec<PageLayoutInfo> {
        self.layout(context, value)
    }
}


#[derive(Serialize, Deserialize)]
struct SequenceLayout {
    elements: Vec<TemplateEntry>
}

#[typetag::serde]
impl Layoutable for SequenceLayout {
    fn get_layouter<'a>(&'a self, _value: &'a LayoutValue) -> Box<dyn super::layout_trait::Layouter<'a> + 'a> {
        let dict = match _value {
            LayoutValue::Dict(d) => d,
            _ => panic!("Expected dict")
        };
        let layouters = self.elements.iter().map(
            |entry| {
                let value = match &entry.key {
                    Some(key) => dict.get(key).unwrap_or(&LayoutValue::None),
                    _ => &LayoutValue::None
                };
                entry.element.get_layouter(value)
            }
        ).collect_vec();

        /*Box::new(SequenceLayouter {
            element_layouter: &layouters,
            curr_layouter_idx: 0
        })*/
        todo!();
    }
}

struct SequenceLayouter<'a> {
    element_layouter: &'a Vec<Box<dyn Layouter<'a> + 'a>>,
    curr_layouter_idx: usize,
}
