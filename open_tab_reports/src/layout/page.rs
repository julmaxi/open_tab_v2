use std::{collections::HashMap, marker::PhantomData, hash::Hash};

use itertools::Itertools;
use serde::{Deserialize, Serialize, ser::{SerializeStruct, SerializeMap}, de::Visitor};

use super::{layout_trait::Layoutable, LayoutContext, LayoutValue, LayoutedElements, DynamicTextBox, FixedImage, LayoutedDocument, FormName};


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

        let form_dict = match dict_val.get("forms") {
            Some(LayoutValue::Dict(d)) => d,
            _ => panic!("Expected dict")
        };

        let vec_val = match dict_val.get("pages") {
            Some(LayoutValue::Vec(v)) => v,
            _ => panic!("Expected Vec")
        };

        for (form_name, form) in self.forms.iter() {
            let elements = form.layout(context, form_dict.get(form_name).unwrap_or(&LayoutValue::None));
            context.graphics_collection.register_form(FormName(form_name.clone()), elements);
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


pub enum Format {
    A4Vertical,
    A4Horizontal,
}

pub struct PageLayoutInfo {
    format: Format,
    elements: LayoutedElements
}


#[typetag::serde(tag = "type")]
pub trait PageLayoutable {
    fn layout_pages(&self, context: &mut LayoutContext, value: &LayoutValue) -> Vec<LayoutedElements>;
}

#[derive(Serialize, Deserialize)]
pub struct SinglePageTemplate {
    pub elements: Vec<(String, Box<dyn Layoutable>)>
}


impl SinglePageTemplate {
    pub fn new(elements: Vec<(String, Box<dyn Layoutable>)>) -> Self {
        Self {
            elements
        }
    }

    pub fn layout(&self, context: &mut LayoutContext, value: &LayoutValue) -> LayoutedElements {
        let mut layouted_elements = LayoutedElements::new();

        for (key, element) in self.elements.iter() {
            let element_value = match value {
                LayoutValue::Dict(dict) => dict.get(key).unwrap_or(&LayoutValue::None),
                _ => &LayoutValue::None
            };

            let layout_result = element.layout(context, element_value);

            layouted_elements.extend(layout_result.objects.into_iter());
        }

        layouted_elements
    }

    pub fn layout_many(&self, context: &mut LayoutContext, values: &LayoutValue) -> Vec<LayoutedElements> {
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
    fn layout_pages(&self, context: &mut LayoutContext, value: &LayoutValue) -> Vec<LayoutedElements> {
        self.layout_many(context, value)
    }
}