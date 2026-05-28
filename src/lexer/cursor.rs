use std::str::CharIndices;

pub(crate) struct Cursor<'a> {
    src_iter: CharIndices<'a>,
}

impl<'a> Cursor<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src_iter: src.char_indices(),
        }
    }

    /// 返回当前游标位置
    pub fn pos(&mut self) -> usize {
        self.src_iter.offset()
    }

    /// 返回当前游标位置的字符
    pub fn peek(&self) -> Option<char> {
        self.src_iter.clone().next().map(|(_, ch)| ch)
    }

    /// 返回游标下一个位置的字符
    pub fn peek_next(&self) -> Option<char> {
        self.src_iter.clone().nth(1).map(|(_, ch)| ch)
    }

    /// 判断是否读到文件末尾
    #[allow(dead_code)]
    pub fn is_eof(&self) -> bool {
        self.src_iter.clone().next().is_none()
    }

    /// 前进一个字符
    pub fn bump(&mut self) -> Option<char> {
        self.src_iter.next().map(|(_, ch)| ch)
    }

    /// 前进到条件
    pub fn eat_while<F: Fn(char) -> bool>(&mut self, pred: F) {
        while let Some(ch) = self.peek() {
            if pred(ch) {
                self.bump();
            } else {
                break;
            }
        }
    }

    /// 如果满足条件则前进
    pub fn eat_if<F: Fn(char) -> bool>(&mut self, pred: F) -> bool {
        match self.peek() {
            Some(ch) if pred(ch) => {
                self.bump();
                true
            }
            _ => false,
        }
    }
}
