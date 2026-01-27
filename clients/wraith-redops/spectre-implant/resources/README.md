# Spectre Resources

## Runner.dll
This is a .NET assembly used by the `powershell` module to execute commands via CLR hosting.

### Building
1. Open `Wraith.Runner.sln` (create it if missing)
2. Create a Class Library named `Wraith.Runner`
3. Implement a class `Runner` with a static method `Run(string cmd) -> int`.
4. Compile as `Release` / `Any CPU`.
5. Copy the output DLL here.

### Current File
The current `Runner.dll` is a placeholder with a valid MZ header signature to allow the implant to compile and pass basic validation checks.
