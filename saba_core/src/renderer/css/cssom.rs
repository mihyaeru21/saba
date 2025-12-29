use core::iter::Peekable;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::renderer::css::token::{CssToken, CssTokenizer};

#[derive(Debug, Clone)]
pub struct CssParser {
    t: Peekable<CssTokenizer>,
}

impl CssParser {
    pub fn new(t: CssTokenizer) -> Self {
        Self { t: t.peekable() }
    }

    pub fn parse_stylesheet(&mut self) -> StyleSheet {
        let mut sheet = StyleSheet::default();
        let rules = self.consume_list_of_rules();
        sheet.set_rules(rules);
        sheet
    }

    fn consume_list_of_rules(&mut self) -> Vec<QualifiedRule> {
        let mut rules = Vec::new();

        loop {
            let Some(token) = self.t.peek() else {
                return rules;
            };
            match *token {
                // AtKeyword トークンが出てきた場合、ほかの CSS をインポートする
                // @import、メディアクエリを表す @media などのルールが始まることを表す
                CssToken::AtKeyword(_) => {
                    let _rule = self.consume_qualified_rule();
                    // しかし、本書のブラウザでは @ から始まるルールはサポートしないので、無視する
                }
                _ => {
                    let rule = self.consume_qualified_rule();
                    match rule {
                        Some(r) => rules.push(r),
                        None => return rules,
                    }
                }
            }
        }
    }

    fn consume_qualified_rule(&mut self) -> Option<QualifiedRule> {
        let mut rule = QualifiedRule::default();

        loop {
            match self.t.peek()? {
                CssToken::OpenCurly => {
                    assert_eq!(self.t.next(), Some(CssToken::OpenCurly));
                    rule.set_declarations(self.consume_list_of_declarations());
                    return Some(rule);
                }
                _ => {
                    rule.set_selector(self.consume_selector());
                }
            }
        }
    }

    fn consume_selector(&mut self) -> Selector {
        let Some(token) = self.t.next() else {
            panic!("should have a token but got None");
        };

        match token {
            CssToken::HashToken(v) => Selector::IdSelector(v[1..].to_string()),
            CssToken::Delim(delim) => {
                if delim == '.' {
                    return Selector::ClassSelector(self.consume_ident());
                }
                panic!("Parse error: {token:?} is an unexpected token.")
            }
            CssToken::Ident(ident) => {
                // a:hover のようなセレクタはタイプセレクタとして扱うため、
                // もしコロン（:）が出てきた場合は宣言ブロックの開始直前までトークンを進める
                if self.t.peek() == Some(&CssToken::Colon) {
                    while self.t.peek() != Some(&CssToken::OpenCurly) {
                        self.t.next();
                    }
                }
                Selector::TypeSelector(ident.to_string())
            }
            CssToken::AtKeyword(_) => {
                // @ から始まるルールを無視するために、宣言ブロックの開始直前までトークンを進める
                while self.t.peek() != Some(&CssToken::OpenCurly) {
                    self.t.next();
                }
                Selector::UnknownSelector
            }
            _ => {
                self.t.next();
                Selector::UnknownSelector
            }
        }
    }

    fn consume_list_of_declarations(&mut self) -> Vec<Declaration> {
        let mut declarations = Vec::new();

        loop {
            let Some(token) = self.t.peek() else {
                return declarations;
            };

            match token {
                CssToken::CloseCurly => {
                    assert_eq!(self.t.next(), Some(CssToken::CloseCurly));
                    return declarations;
                }
                CssToken::SemiColon => {
                    assert_eq!(self.t.next(), Some(CssToken::SemiColon));
                    // ひとつの宣言が終了。何もしない
                }
                CssToken::Ident(_) => {
                    if let Some(declaration) = self.consume_declaration() {
                        declarations.push(declaration);
                    }
                }
                _ => {
                    self.t.next();
                }
            }
        }
    }

    fn consume_declaration(&mut self) -> Option<Declaration> {
        self.t.peek()?;

        let mut declaration = Declaration::default();
        declaration.set_property(self.consume_ident());

        match self.t.next()? {
            CssToken::Colon => {}
            _ => return None,
        }

        declaration.set_value(self.consume_component_value());

        Some(declaration)
    }

    fn consume_ident(&mut self) -> String {
        let Some(token) = self.t.next() else {
            panic!("should have a token but got None");
        };

        match token {
            CssToken::Ident(ref ident) => ident.to_string(),
            _ => {
                panic!("Parse erroe {token:?} is an unexpected token.")
            }
        }
    }

    fn consume_component_value(&mut self) -> ComponentValue {
        self.t
            .next()
            .expect("should have a token in consume_component_value")
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StyleSheet {
    pub rules: Vec<QualifiedRule>,
}

impl StyleSheet {
    pub fn set_rules(&mut self, rules: Vec<QualifiedRule>) {
        self.rules = rules;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedRule {
    pub selector: Selector,
    pub declarations: Vec<Declaration>,
}

impl Default for QualifiedRule {
    fn default() -> Self {
        Self {
            selector: Selector::TypeSelector("".to_string()),
            declarations: Vec::new(),
        }
    }
}

impl QualifiedRule {
    pub fn set_selector(&mut self, selector: Selector) {
        self.selector = selector;
    }

    pub fn set_declarations(&mut self, declarations: Vec<Declaration>) {
        self.declarations = declarations;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    TypeSelector(String),
    ClassSelector(String),
    IdSelector(String),
    UnknownSelector,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub property: String,
    pub value: ComponentValue,
}

impl Default for Declaration {
    fn default() -> Self {
        Self {
            property: String::new(),
            value: ComponentValue::Ident(String::new()),
        }
    }
}

impl Declaration {
    pub fn set_property(&mut self, property: String) {
        self.property = property;
    }

    pub fn set_value(&mut self, value: ComponentValue) {
        self.value = value;
    }
}

pub type ComponentValue = CssToken;

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn test_empty() {
        let style = "".to_string();
        let t = CssTokenizer::new(style);
        let cssom = CssParser::new(t).parse_stylesheet();

        assert_eq!(cssom.rules.len(), 0);
    }

    #[test]
    fn test_one_rule() {
        let style = "p { color: red; }".to_string();
        let t = CssTokenizer::new(style);
        let cssom = CssParser::new(t).parse_stylesheet();

        let mut rule = QualifiedRule::default();
        rule.set_selector(Selector::TypeSelector("p".to_string()));
        let mut declaration = Declaration::default();
        declaration.set_property("color".to_string());
        declaration.set_value(ComponentValue::Ident("red".to_string()));
        rule.set_declarations(vec![declaration]);

        let expected = [rule];
        assert_eq!(cssom.rules.len(), expected.len());
        for (r, e) in cssom.rules.iter().zip(expected.iter()) {
            assert_eq!(r, e);
        }
    }

    #[test]
    fn test_id_selector() {
        let style = "#id { color: red; }".to_string();
        let t = CssTokenizer::new(style);
        let cssom = CssParser::new(t).parse_stylesheet();

        let mut rule = QualifiedRule::default();
        rule.set_selector(Selector::IdSelector("id".to_string()));
        let mut declaration = Declaration::default();
        declaration.set_property("color".to_string());
        declaration.set_value(ComponentValue::Ident("red".to_string()));
        rule.set_declarations(vec![declaration]);

        let expected = [rule];
        assert_eq!(cssom.rules.len(), expected.len());
        for (r, e) in cssom.rules.iter().zip(expected.iter()) {
            assert_eq!(r, e);
        }
    }

    #[test]
    fn test_class_selector() {
        let style = ".class { color: red; }".to_string();
        let t = CssTokenizer::new(style);
        let cssom = CssParser::new(t).parse_stylesheet();

        let mut rule = QualifiedRule::default();
        rule.set_selector(Selector::ClassSelector("class".to_string()));
        let mut declaration = Declaration::default();
        declaration.set_property("color".to_string());
        declaration.set_value(ComponentValue::Ident("red".to_string()));
        rule.set_declarations(vec![declaration]);

        let expected = [rule];
        assert_eq!(cssom.rules.len(), expected.len());
        for (r, e) in cssom.rules.iter().zip(expected.iter()) {
            assert_eq!(r, e);
        }
    }

    #[test]
    fn test_multiple_rule() {
        let style = r#"p { content: "Hey"; } h1 { font-size: 40; color: blue; }"#.to_string();
        let t = CssTokenizer::new(style);
        let cssom = CssParser::new(t).parse_stylesheet();

        let mut rule1 = QualifiedRule::default();
        rule1.set_selector(Selector::TypeSelector("p".to_string()));
        let mut declaration = Declaration::default();
        declaration.set_property("content".to_string());
        declaration.set_value(ComponentValue::StringToken("Hey".to_string()));
        rule1.set_declarations(vec![declaration]);

        let mut rule2 = QualifiedRule::default();
        rule2.set_selector(Selector::TypeSelector("h1".to_string()));
        let mut declaration2 = Declaration::default();
        declaration2.set_property("font-size".to_string());
        declaration2.set_value(ComponentValue::Number(40.0));
        let mut declaration3 = Declaration::default();
        declaration3.set_property("color".to_string());
        declaration3.set_value(ComponentValue::Ident("blue".to_string()));
        rule2.set_declarations(vec![declaration2, declaration3]);

        let expected = [rule1, rule2];
        assert_eq!(cssom.rules.len(), expected.len());
        for (r, e) in cssom.rules.iter().zip(expected.iter()) {
            assert_eq!(r, e);
        }
    }
}
