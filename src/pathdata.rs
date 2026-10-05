//! Path data to the absolute end points of its commands, the way connector
//! routes are read: horizontal, vertical and closing commands become lines,
//! curves and arcs contribute their end points, and malformed data yields
//! no points at all.

use crate::geometry::{Point, point};

pub fn end_points(data: &str) -> Vec<Point> {
    parse(data).unwrap_or_default()
}

#[derive(Clone, Copy)]
enum Token {
    Command(char),
    Number(f64),
}

fn parse(data: &str) -> Option<Vec<Point>> {
    let tokens = tokenize(data)?;
    let mut points = Vec::new();
    let mut current = point(0.0, 0.0);
    let mut start: Option<Point> = None;
    let mut index = 0;
    let mut command: Option<char> = None;

    while index < tokens.len() {
        let letter = match tokens[index] {
            Token::Command(letter) => {
                index += 1;
                letter
            }
            Token::Number(_) => match command {
                // Coordinates after a moveto continue as linetos.
                Some('M') => 'L',
                Some('m') => 'l',
                Some(previous) if !matches!(previous, 'Z' | 'z') => previous,
                _ => return None,
            },
        };
        if start.is_none() && !matches!(letter, 'M' | 'm') {
            return None;
        }
        let relative = letter.is_ascii_lowercase();
        let arity = match letter.to_ascii_uppercase() {
            'M' | 'L' | 'T' => 2,
            'H' | 'V' => 1,
            'C' => 6,
            'S' | 'Q' => 4,
            'A' => 7,
            'Z' => 0,
            _ => return None,
        };
        let arguments: Vec<f64> = tokens
            .get(index..index + arity)?
            .iter()
            .map(|token| match token {
                Token::Number(value) => Some(*value),
                Token::Command(_) => None,
            })
            .collect::<Option<_>>()?;
        index += arity;
        let offset = |x: f64, y: f64| {
            if relative {
                point(current.x + x, current.y + y)
            } else {
                point(x, y)
            }
        };
        let end = match letter.to_ascii_uppercase() {
            'M' => {
                let end = offset(arguments[0], arguments[1]);
                start = Some(end);
                end
            }
            'L' | 'T' => offset(arguments[0], arguments[1]),
            'H' => point(
                if relative {
                    current.x + arguments[0]
                } else {
                    arguments[0]
                },
                current.y,
            ),
            'V' => point(
                current.x,
                if relative {
                    current.y + arguments[0]
                } else {
                    arguments[0]
                },
            ),
            'C' => offset(arguments[4], arguments[5]),
            'S' | 'Q' => offset(arguments[2], arguments[3]),
            'A' => offset(arguments[5], arguments[6]),
            _ => start?,
        };
        points.push(end);
        current = end;
        command = Some(match letter {
            'M' => 'L',
            'm' => 'l',
            other => other,
        });
        if matches!(letter, 'M' | 'm') {
            command = Some(letter);
        }
    }
    Some(points)
}

fn tokenize(data: &str) -> Option<Vec<Token>> {
    let characters: Vec<char> = data.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    let mut arc_argument: Option<usize> = None;
    while index < characters.len() {
        let character = characters[index];
        if character.is_whitespace() || character == ',' {
            index += 1;
            continue;
        }
        if character.is_ascii_alphabetic() && !matches!(character, 'e' | 'E') {
            arc_argument = matches!(character, 'A' | 'a').then_some(0);
            tokens.push(Token::Command(character));
            index += 1;
            continue;
        }
        // Arc flags are single digits that may run into the next number.
        if let Some(argument) = arc_argument {
            let position = argument % 7;
            if (position == 3 || position == 4) && matches!(character, '0' | '1') {
                tokens.push(Token::Number(if character == '1' { 1.0 } else { 0.0 }));
                arc_argument = Some(argument + 1);
                index += 1;
                continue;
            }
        }
        let (value, length) = read_number(&characters[index..])?;
        tokens.push(Token::Number(value));
        arc_argument = arc_argument.map(|argument| argument + 1);
        index += length;
    }
    Some(tokens)
}

/// Reads one number: sign, digits, fraction and exponent, stopping where a
/// second sign or decimal point starts the next number.
fn read_number(characters: &[char]) -> Option<(f64, usize)> {
    let mut length = 0;
    if matches!(characters.first(), Some('+' | '-')) {
        length += 1;
    }
    let digits_from = |from: usize| characters[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let integer = digits_from(length);
    length += integer;
    let mut fraction = 0;
    if characters.get(length) == Some(&'.') {
        fraction = digits_from(length + 1);
        length += 1 + fraction;
    }
    if integer == 0 && fraction == 0 {
        return None;
    }
    if matches!(characters.get(length), Some('e' | 'E')) {
        let mut exponent = length + 1;
        if matches!(characters.get(exponent), Some('+' | '-')) {
            exponent += 1;
        }
        let exponent_digits = digits_from(exponent);
        if exponent_digits == 0 {
            return None;
        }
        length = exponent + exponent_digits;
    }
    let text: String = characters[..length].iter().collect();
    text.parse::<f64>().ok().map(|value| (value, length))
}
