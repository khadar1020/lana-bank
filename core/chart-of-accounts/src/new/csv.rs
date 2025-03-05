use csv::{ReaderBuilder, Trim};
use std::io::Cursor;

use super::primitives::{
    AccountCategory, AccountCodeSection, AccountCodeSectionParseError, AccountSpec,
};

use thiserror::Error;

#[derive(Error, Debug)]
#[error("CsvParseError")]
pub struct CsvParseError;

pub struct CsvParser {
    data: String,
}
impl CsvParser {
    pub fn new(data: String) -> Self {
        Self { data }
    }

    pub fn account_specs(self) -> Result<Vec<AccountSpec>, CsvParseError> {
        let mut rdr = ReaderBuilder::new()
            .flexible(true)
            .trim(Trim::All)
            .has_headers(false)
            .from_reader(Cursor::new(self.data));

        let mut specs: Vec<AccountSpec> = vec![];
        for result in rdr.records() {
            match result {
                Ok(record) => {
                    let mut initial_empty = true;
                    let mut sections = vec![];
                    if record.iter().all(|field| field.is_empty()) {
                        continue;
                    }

                    for (idx, field) in record.iter().enumerate() {
                        if let Ok(category) = field.parse::<AccountCategory>() {
                            if let Some(s) = specs.last() {
                                if s.code.is_parent(&sections) {
                                    specs.push(AccountSpec::new(
                                        Some(s.code.clone()),
                                        sections,
                                        category,
                                    ));
                                    break;
                                }
                            }
                            specs.push(AccountSpec::new(None, sections, category));
                            break;
                        }
                        match field.parse::<AccountCodeSection>() {
                            Ok(section) => {
                                initial_empty = false;
                                sections.push(section)
                            }
                            Err(AccountCodeSectionParseError::Empty) if initial_empty => {
                                sections.push(
                                    specs
                                        .last()
                                        .expect("No parent")
                                        .code
                                        .section(idx)
                                        .expect("No parent section")
                                        .clone(),
                                );
                            }
                            _ => {
                                continue;
                            }
                        }
                    }
                }
                Err(e) => eprintln!("Error reading record: {}", e),
            }
        }

        Ok(specs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_one_line() {
        let data = r#"1,,,Assets"#;
        let parser = CsvParser::new(data.to_string());
        let specs = parser.account_specs().unwrap();
        assert_eq!(specs.len(), 1);
    }

    #[test]
    fn parse_two_lines() {
        let data = r#"
        1,,,Assets ,,
        ,,,,,
        11,,,Assets,,
        ,,,,,
        "#;
        let parser = CsvParser::new(data.to_string());
        let specs = parser.account_specs().unwrap();
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].code.len_sections(), 1);
        assert_eq!(Some(&specs[0].code), specs[1].parent.as_ref());
    }

    #[test]
    fn parse_child_with_empty_top_section() {
        let data = r#"
        1,,,Assets ,,
        ,,,,,
        11,,,Assets,,
        ,,,,,
            ,01,,Effective,,
        ,,0101,Central Office,
        "#;
        let parser = CsvParser::new(data.to_string());
        let specs = parser.account_specs().unwrap();
        assert_eq!(specs.len(), 4);

        assert_eq!(specs[2].code.len_sections(), 2);
        assert_eq!(Some(&specs[1].code), specs[2].parent.as_ref());

        assert_eq!(specs[3].code.len_sections(), 3);
        assert_eq!(Some(&specs[2].code), specs[3].parent.as_ref());

        assert_eq!(&specs[3].code.to_string(), "11.01.0101");
    }
}
