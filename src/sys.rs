use std::vec;

extern "C" {
    pub fn gethostname(name: *mut libc::c_char, size: libc::size_t) -> libc::c_int;
}

pub fn hostname() -> anyhow::Result<String> {
    // If the hosname is bigger than maxlen, it'll be truncated
    let maxlen: usize = 128;
    let mut buf: Vec<u8> = vec![0; maxlen];

    let err = unsafe { gethostname(buf.as_mut_ptr() as *mut i8, maxlen) };
    if err != 0 {
        anyhow::bail!("oops, gethostname failed: error {}", err);
    }

    // find the first 0 byte (i.e. just after the data that gethostname wrote)
    let actual_len = buf.iter().position(|byte| *byte == 0).unwrap_or(maxlen);

    // trim the hostname to the actual data written
    buf.truncate(actual_len);

    // Turn the bytes to a rust String
    let result = String::from_utf8(buf)?;

    Ok(result)
}
