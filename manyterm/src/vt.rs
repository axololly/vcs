#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum Token<'input> {
    /// Printable character.
    Text(char),

    /// Control character.
    Control(char),

    /// Escape character (`ESCx`)
    Esc(char),

    /// `Csi(header, parameters, footer)`
    Csi(Option<char>, Vec<u16>, Option<char>),
    
    /// `Osc(data)`
    Osc(&'input str),

    /// `Dcs(data)`
    Dcs(&'input str),

    /// `Ss3(data)`
    Ss3(char)
}

pub(crate) struct VTStream {
    pub input: Vec<u8>,
    pub offset: usize
}

#[allow(dead_code)]
impl VTStream {
    pub fn new(input: Vec<u8>) -> VTStream {
        VTStream {
            input,
            offset: 0
        }
    }

    pub fn is_done(&self) -> bool {
        self.input[self.offset..].is_empty()
    }

    #[allow(
        clippy::should_implement_trait,
        reason = "VTStream is a lending iterator which does not return owned values."
    )]
    pub fn next(&mut self) -> Option<Token<'_>> {
        let input = &self.input[self.offset..];

        if input.is_empty() {
            return None;
        }

        self.offset += 1;

        Some(match input[0] {
            // Start of escape sequence
            0x1b => self.parse_escape(),

            // Control character
            b @ (0x00..=0x1F | 0x7F) => Token::Control(b as char),

            // Normal printable character
            b => Token::Text(b as char)
        })
    }

    fn parse_escape(&mut self) -> Token<'_> {
        let input = &self.input[self.offset..];

        if input.is_empty() {
            return Token::Control('\x1b');
        }

        self.offset += 1;

        match input[0] {
            // CSI sequence
            b'[' => self.parse_csi().unwrap_or(Token::Esc('[')),

            // OSC sequence
            b']' => self.parse_osc().unwrap_or(Token::Esc(']')),

            // SS3 sequence
            b'O' => {
                if let Some(&next) = input.get(1) {
                    self.offset += 1;

                    Token::Ss3(next as char)
                }
                else {
                    Token::Esc('O')
                }
            },

            // DCS sequence
            b'P' => self.parse_dcs().unwrap_or(Token::Esc('P')),

            // ESCx sequence
            b @ 0x20..0x7F => Token::Esc(b as char),

            _ => todo!()
        }
    }

    fn parse_csi(&mut self) -> Option<Token<'_>> {
        if self.offset >= self.input.len() {
            return None;
        }

        let first = self.input[self.offset];

        let header = match first {
            b'?' | b'!' | b'<' => {
                self.offset += 1;
                
                Some(first as char)
            }
            _ => None
        };

        let mut footer: Option<char> = None;

        let mut params: Vec<u16> = Vec::with_capacity(16);
        let mut param_index = 0;

        let input = &self.input[self.offset ..];

        // Look ahead for the ending character.
        for (index, &byte) in input.iter().enumerate() {
            match byte {
                // This is part of a current parameter.
                b'0'..=b'9' => {
                    let digit = (byte as u16) - (b'0' as u16);

                    // Need to introduce another parameter
                    if param_index + 1 > params.len() {
                        params.push(digit);
                        continue;
                    }

                    let param = params[param_index];
                    
                    params[param_index] = param.saturating_mul(10).saturating_add(digit);
                }

                // A new parameter is being introduced/
                b';' => {
                    param_index += 1;
                }

                // This is the footer that we are looking for.
                0x40..0x7F => {
                    footer = Some(byte as char);

                    self.offset += index + 1;
                    
                    break;
                }

                // We have an invalid character which means this is a broken
                // and therefore invalid sequence.
                _ => break
            }
        }

        // No token was available so there's nothing to return
        if header.is_none() && params.is_empty() && footer.is_none() {
            None
        }
        // Some data was left, so this is technically a valid token.
        else {
            Some(Token::Csi(header, params, footer))
        }
    }

    // Abstraction of OSC and DCS parsing logic.
    // This just gets the necessary string.
    fn parse_x(&mut self) -> Option<&str> {
        let input = &self.input[self.offset ..];

        if input.is_empty() {
            return None;
        }

        let mut end = input.len();
        
        for (index, &byte) in input.iter().enumerate() {
            match byte {
                0x07 => {
                    end = index;

                    self.offset += index;

                    break;
                }

                0x1B if index + 1 < input.len() && input[index + 1] == b'\\' => {
                    end = index;

                    self.offset += index + 2;

                    break;
                }

                _ => {}
            }
        }

        Some(str::from_utf8(&input[..end]).unwrap())
    }

    fn parse_osc(&mut self) -> Option<Token<'_>> {
        self.parse_x().map(Token::Osc)
    }

    fn parse_dcs(&mut self) -> Option<Token<'_>> {
        self.parse_x().map(Token::Dcs)
    }
}