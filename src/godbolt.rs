use reqwest::Error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Language {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
struct CompileRequest<'a> {
    source: &'a str,
    options: CompileOptions,
}

#[derive(Debug, Serialize)]
struct CompileOptions {}

#[derive(Debug, Deserialize)]
struct AsmLine {
    text: String,
}

#[derive(Debug, Deserialize)]
struct StderrLine {
    text: String,
}

#[derive(Debug, Deserialize)]
struct CompileResponse {
    asm: Vec<AsmLine>,
    stderr: Vec<StderrLine>,
    #[allow(dead_code)]
    code: i32,
}

#[derive(Debug)]
pub enum CompilationOutput {
    Assembly(String),
    Stderr(String),
}

#[derive(Debug, Deserialize)]
pub struct Compiler {
    pub id: String,
    pub name: String,
    pub semver: String,
}

#[derive(Debug, Serialize)]
struct ExecuteFilterOptions {
    execute: bool,
}

#[derive(Debug, Serialize)]
struct ExecuteParameters<'a> {
    stdin: &'a str,
}

#[derive(Debug, Serialize)]
struct ExecuteOptions<'a> {
    filters: ExecuteFilterOptions,
    #[serde(rename = "executeParameters")]
    execute_parameters: ExecuteParameters<'a>,
}

#[derive(Debug, Serialize)]
struct ExecuteRequest<'a> {
    source: &'a str,
    options: ExecuteOptions<'a>,
}


#[derive(Debug, Deserialize)]
struct OutputLine {
    text: String,
}

#[derive(Debug, Deserialize)]
struct BuildResult {
    code: i32,
    stderr: Vec<OutputLine>,
}

#[derive(Debug, Deserialize)]
struct ExecResult {
    code: i32,
    stdout: Vec<OutputLine>,
    stderr: Vec<OutputLine>,
    #[serde(rename = "buildResult")]
    build_result: BuildResult,
}

#[derive(Debug, Deserialize)]
struct ExecuteResponse {
    #[serde(rename = "execResult")]
    exec_result: Option<ExecResult>,
}


#[derive(Debug)]
pub enum ExecutionOutput {
    BuildFailure(String),
    ExecutionSuccess {
        stdout: String,
        stderr: String,
        exit_code: i32,
    },
    ApiError(String),
}

const GODBOLT_URL: &str = "https://godbolt.org";

fn route(path: &str) -> String {
    format!("{GODBOLT_URL}/api/{path}")
}

/// # Description
/// Compiles the given source code using the specified compiler ID.
///
/// # Arguments
/// * `compiler_id` - The ID of the compiler (e.g., "g122" for GCC 12.2).
/// * `code` - The source code to compile.
///
/// # Returns
/// A `Result` which is:
/// * `Ok(CompilationOutput)` on a successful API call. The enum will contain
///   either the assembly or the compiler's stderr.
/// * `Err(reqwest::Error)` if a network or deserialization error occurs.
pub async fn compile(compiler_id: &str, code: &str) -> Result<CompilationOutput, Error> {
    log::info!("Received '{code}' to compile with {compiler_id}.");

    let request_url = route(&format!("compiler/{compiler_id}/compile"));

    let request_body = CompileRequest {
        source: code,
        options: CompileOptions {},
    };

    let client = reqwest::Client::new();
    let res = client
        .post(request_url)
        .header("Accept", "application/json")
        .query(&[("fields", "id,name,semver")])
        .json(&request_body)
        .send()
        .await?;

    let compile_res: CompileResponse = res.json().await?;

    if !compile_res.stderr.is_empty() {
        let error_output = compile_res
            .stderr
            .into_iter()
            .map(|line| line.text)
            .collect::<Vec<String>>()
            .join("\n");
        Ok(CompilationOutput::Stderr(error_output))
    } else {
        let assembly_output = compile_res
            .asm
            .into_iter()
            .map(|line| line.text)
            .collect::<Vec<String>>()
            .join("\n");
        Ok(CompilationOutput::Assembly(assembly_output))
    }
}

/// # Description
/// Compiles and then executes the given code with specified stdin.
///
/// # Returns
/// A `Result` which is:
/// * `Ok(ExecutionOutput)` on a successful API call.
/// * `Err(reqwest::Error)` if a network or deserialization error occurs.
pub async fn execute(
    compiler_id: &str,
    code: &str,
    stdin: &str,
) -> Result<ExecutionOutput, Error> {
    log::info!("Executing '{code}' with compiler '{compiler_id}' and stdin '{stdin}'");

    let request_url = route(&format!("compiler/{compiler_id}/compile"));

    let request_body = ExecuteRequest {
        source: code,
        options: ExecuteOptions {
            filters: ExecuteFilterOptions { execute: true }, // This is the key
            execute_parameters: ExecuteParameters { stdin },
        },
    };

    let client = reqwest::Client::new();
    let res = client
        .post(request_url)
        .header("Accept", "application/json")
        .json(&request_body)
        .send()
        .await?;

    let exec_res: ExecuteResponse = res.json().await?;

    if let Some(result) = exec_res.exec_result {
        if result.build_result.code != 0 {
            let build_errors = result
                .build_result
                .stderr
                .into_iter()
                .map(|line| line.text)
                .collect::<Vec<String>>()
                .join("\n");
            Ok(ExecutionOutput::BuildFailure(build_errors))
        } else {
            let stdout = result
                .stdout
                .into_iter()
                .map(|line| line.text)
                .collect::<Vec<String>>()
                .join("\n");
            let stderr = result
                .stderr
                .into_iter()
                .map(|line| line.text)
                .collect::<Vec<String>>()
                .join("\n");

            Ok(ExecutionOutput::ExecutionSuccess {
                stdout,
                stderr,
                exit_code: result.code,
            })
        }
    } else {
        // This case handles if the API call succeeded but did not return an exec result.
        Ok(ExecutionOutput::ApiError(
            "API did not return an execution result.".to_string(),
        ))
    }
}

/// # Description
/// Retrieves a list of all supported compilers for a specific language.
///
/// # Arguments
/// * `language_id` - The ID of the language (e.g., "rust", "cpp", "csharp").
///
/// # Returns
/// A `Result` which is:
/// * `Ok(Vec<Compiler>)` on a successful API call, containing the list of compilers.
/// * `Err(reqwest::Error)` if a network or deserialization error occurs.
pub async fn compilers_for_language(language_id: &str) -> Result<Vec<Compiler>, Error> {
    let request_url = route(&format!("compilers/{}", language_id));
    log::info!(
        "Requesting compilers for language '{}' from URL: {}",
        language_id,
        request_url
    );

    let client = reqwest::Client::new();
    let res = client
        .get(request_url)
        .header("Accept", "application/json")
        .send()
        .await?;

    let compilers: Vec<Compiler> = res.json().await?;

    Ok(compilers)
}

pub async fn languages() -> Result<Vec<Language>, Error> {
    let request_url = route("languages");
    let client = reqwest::Client::new();
    let res = client
        .get(request_url)
        .header("Accept", "application/json")
        .send()
        .await?;
    let langs: Vec<Language> = res.json().await?;

    Ok(langs)
}

