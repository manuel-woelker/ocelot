use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_base::source_file::SourceFile;
use ocelot_pal::pal::PalHandle;

#[derive(Debug, Clone)]
pub struct Engine {
    pal: PalHandle,
}

impl Engine {
    pub fn new(pal: PalHandle) -> Self {
        Self { pal }
    }

    pub fn run_script(&self, path: impl Into<FilePath>) -> OcelotResult<()> {
        let source_file = self.load_source_file(path.into())?;
        let script = ocelot_parser::parse_script::parse_script(&source_file)?;
        ocelot_interpreter::interpret_script::interpret_script(&script, &*self.pal)?;
        Ok(())
    }

    fn load_source_file(&self, path: FilePath) -> OcelotResult<SourceFile> {
        let source = self.pal.read_file_to_string(&path)?;
        Ok(SourceFile::new(path, source))
    }
}

#[cfg(test)]
mod tests {
    use super::Engine;
    use expect_test::expect;
    use ocelot_pal::pal::PalHandle;
    use ocelot_pal::pal_mock::PalMock;

    #[test]
    fn run_script_reads_and_executes_a_file() {
        let pal = PalMock::new();
        pal.set_file("examples/hello_world.ocelot", "println(\"hello, world\");");

        let engine = Engine::new(PalHandle::new(pal.clone()));

        engine.run_script("examples/hello_world.ocelot").unwrap();

        expect![[r#"
            READ FILE: examples/hello_world.ocelot
            PRINT: hello, world

        "#]]
        .assert_eq(&pal.get_effects());
        assert_eq!(pal.take_printed_output(), "hello, world\n");
    }
}
