pub trait ExtResult<T> {
    fn get_or(self, err: &'static str) -> anyhow::Result<T>;
    fn get(self) -> anyhow::Result<T>;
}

impl<T> ExtResult<T> for Option<T> {
    fn get_or(self, err: &'static str) -> anyhow::Result<T> {
        self.ok_or(anyhow::anyhow!(err))
    }

    fn get(self) -> anyhow::Result<T> {
        self.get_or("")
    }
}

impl<T, E> ExtResult<T> for Result<T, E> {
    fn get_or(self, err: &'static str) -> anyhow::Result<T> {
        self.map_err(|_| anyhow::anyhow!(err))
    }

    fn get(self) -> anyhow::Result<T> {
        self.get_or("")
    }
}
