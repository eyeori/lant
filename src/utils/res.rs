pub trait ExtResult<T> {
    fn or_err(self, err: &'static str) -> anyhow::Result<T>;
    fn ok(self) -> anyhow::Result<T>;
}

impl<T> ExtResult<T> for Option<T> {
    fn or_err(self, err: &'static str) -> anyhow::Result<T> {
        self.ok_or(anyhow::anyhow!(err))
    }

    fn ok(self) -> anyhow::Result<T> {
        self.or_err("")
    }
}
