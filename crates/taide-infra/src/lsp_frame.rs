const HEADER_SEPARATOR: &[u8] = b"\r\n\r\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportFailure {
    InvalidLimits,
    AllocationFailed,
    HeaderTooLarge,
    BodyTooLarge,
    MalformedHeader,
    UnsupportedCharset,
    InvalidUtf8,
    TruncatedFrame,
    ConsumerUnavailable,
    ReadFailed,
    WriteFailed,
}

#[derive(Clone, Copy, Debug)]
pub struct FrameLimits {
    header_bytes: usize,
    body_bytes: usize,
}

impl FrameLimits {
    pub fn new(header_bytes: usize, body_bytes: usize) -> Result<Self, TransportFailure> {
        if header_bytes < HEADER_SEPARATOR.len() || body_bytes == 0 {
            return Err(TransportFailure::InvalidLimits);
        }
        let total = header_bytes.checked_add(body_bytes).ok_or(TransportFailure::InvalidLimits)?;
        isize::try_from(total).map_err(|_| TransportFailure::InvalidLimits)?;
        Ok(Self { header_bytes, body_bytes })
    }

    pub fn header_bytes(self) -> usize {
        self.header_bytes
    }

    pub fn body_bytes(self) -> usize {
        self.body_bytes
    }
}

pub struct BoundedMessageBuffer {
    limits: FrameLimits,
    header: Vec<u8>,
    body: Vec<u8>,
    expected_body: Option<usize>,
    failure: Option<TransportFailure>,
}

impl BoundedMessageBuffer {
    pub fn new(limits: FrameLimits) -> Result<Self, TransportFailure> {
        let mut header = Vec::new();
        header
            .try_reserve_exact(limits.header_bytes)
            .map_err(|_| TransportFailure::AllocationFailed)?;
        Ok(Self {
            limits,
            header,
            body: Vec::new(),
            expected_body: None,
            failure: None,
        })
    }

    pub fn retained_bytes(&self) -> usize {
        self.header.len() + self.body.len()
    }

    pub fn allocated_capacity(&self) -> usize {
        self.header.capacity() + self.body.capacity()
    }

    pub fn push<F>(&mut self, chunk: &[u8], on_message: F) -> Result<(), TransportFailure>
    where
        F: FnMut(String) -> Result<(), TransportFailure>,
    {
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        match self.push_inner(chunk, on_message) {
            Ok(()) => Ok(()),
            Err(failure) => self.fail(failure),
        }
    }

    pub fn finish(&mut self) -> Result<(), TransportFailure> {
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        if !self.header.is_empty() || self.expected_body.is_some() {
            return self.fail(TransportFailure::TruncatedFrame);
        }
        Ok(())
    }

    fn push_inner<F>(&mut self, mut chunk: &[u8], mut on_message: F) -> Result<(), TransportFailure>
    where
        F: FnMut(String) -> Result<(), TransportFailure>,
    {
        while !chunk.is_empty() {
            if let Some(length) = self.expected_body {
                let count = (length - self.body.len()).min(chunk.len());
                self.body.extend_from_slice(&chunk[..count]);
                chunk = &chunk[count..];
                if self.body.len() == length {
                    let body = std::mem::take(&mut self.body);
                    self.expected_body = None;
                    let message = String::from_utf8(body).map_err(|_| TransportFailure::InvalidUtf8)?;
                    on_message(message)?;
                }
                continue;
            }
            if self.header.len() >= self.limits.header_bytes {
                return Err(TransportFailure::HeaderTooLarge);
            }
            self.header.push(chunk[0]);
            chunk = &chunk[1..];
            if self.header.ends_with(HEADER_SEPARATOR) {
                let length = parse_header(&self.header, self.limits)?;
                self.header.clear();
                self.body
                    .try_reserve_exact(length)
                    .map_err(|_| TransportFailure::AllocationFailed)?;
                self.expected_body = Some(length);
            }
        }
        Ok(())
    }

    fn fail(&mut self, failure: TransportFailure) -> Result<(), TransportFailure> {
        self.failure = Some(failure);
        self.header.clear();
        self.body = Vec::new();
        self.expected_body = None;
        Err(failure)
    }
}

fn parse_header(header: &[u8], limits: FrameLimits) -> Result<usize, TransportFailure> {
    if !header.is_ascii() {
        return Err(TransportFailure::MalformedHeader);
    }
    let text = std::str::from_utf8(header).map_err(|_| TransportFailure::MalformedHeader)?;
    let text = text.strip_suffix("\r\n\r\n").ok_or(TransportFailure::MalformedHeader)?;
    let mut length = None;
    let mut has_content_type = false;
    for line in text.split("\r\n") {
        let (name, value) = line.split_once(':').ok_or(TransportFailure::MalformedHeader)?;
        if name.is_empty()
            || !name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            || value.bytes().any(|byte| byte.is_ascii_control() && byte != b'\t')
        {
            return Err(TransportFailure::MalformedHeader);
        }
        let value = value.trim();
        if name.eq_ignore_ascii_case("Content-Length") {
            if length.is_some() || value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(TransportFailure::MalformedHeader);
            }
            let parsed = value.parse::<usize>().map_err(|_| TransportFailure::MalformedHeader)?;
            if parsed == 0 {
                return Err(TransportFailure::MalformedHeader);
            }
            if parsed > limits.body_bytes {
                return Err(TransportFailure::BodyTooLarge);
            }
            length = Some(parsed);
        }
        if name.eq_ignore_ascii_case("Content-Type") {
            if has_content_type {
                return Err(TransportFailure::MalformedHeader);
            }
            has_content_type = true;
            for parameter in value.split(';').skip(1) {
                let (name, charset) = parameter.trim().split_once('=').ok_or(TransportFailure::MalformedHeader)?;
                if name.trim().eq_ignore_ascii_case("charset")
                    && !charset.trim().eq_ignore_ascii_case("utf8")
                    && !charset.trim().eq_ignore_ascii_case("utf-8")
                {
                    return Err(TransportFailure::UnsupportedCharset);
                }
            }
        }
    }
    length.ok_or(TransportFailure::MalformedHeader)
}
