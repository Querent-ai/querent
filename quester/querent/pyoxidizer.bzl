def make_exe():
    # Obtain the default PythonDistribution for our build target. We link
    # this distribution into our produced executable and extract the Python
    # standard library from it.
    dist = default_python_distribution()

    # This function creates a `PythonPackagingPolicy` instance, which
    # influences how executables are built and how resources are added to
    # the executable. You can customize the default behavior by assigning
    # to attributes and calling functions.
    policy = dist.make_python_packaging_policy()

    # Attempt to add resources relative to the built binary when
    # `resources_location` fails.
    #
    # Refs: https://github.com/indygreg/PyOxidizer/issues/438
    policy.resources_location_fallback = "filesystem-relative:prefix"

    # This variable defines the configuration of the embedded Python
    # interpreter. By default, the interpreter will run a Python REPL
    # using settings that are appropriate for an "isolated" run-time
    # environment.
    #
    # The configuration of the embedded Python interpreter can be modified
    # by setting attributes on the instance. Some of these are
    # documented below.
    python_config = dist.make_python_interpreter_config()

    # Evaluate a string as Python code when the interpreter starts.
    python_config.run_command = "from flameshow.main import main; main()"

    # Produce a PythonExecutable from a Python distribution, embedded
    # resources, and other options. The returned object represents the
    # standalone executable that will be built.
    exe = dist.to_python_executable(
        name="pyquerent",

        # If no argument passed, the default `PythonPackagingPolicy` for the
        # distribution is used.
        packaging_policy=policy,

        # If no argument passed, the default `PythonInterpreterConfig` is used.
        config=python_config,
    )

    # Invoke `pip install` with our Python distribution to install a single package.
    # `pip_install()` returns objects representing installed files.
    # `add_python_resources()` adds these objects to the binary, with a load
    # location as defined by the packaging policy's resource location
    # attributes.
    exe.add_python_resources(exe.pip_install([
        "aiofiles==23.2.1",
        "aiohttp==3.9.3",
        "attrs==23.1.0",
        "azure-storage-blob==12.19.0",
        "beautifulsoup4==4.12.3",
        "boto3==1.26.146",
        "botocore==1.29.146",
        "bs4==0.0.1",
        "cachetools==5.3.3",
        "coverage==7.3.3",
        "dropbox==11.36.2",
        "faiss-cpu==1.7.4",
        "ffmpeg-python==0.2.0",
        "gensim==4.3.2",
        "google-api-python-client==2.105.0",
        "google-cloud-storage==2.14.0",
        "google-cloud-storage==2.14.0",
        "google-cloud-storage==2.14.0",
        "hdbscan==0.8.33",
        "jira==3.6.0",
        "jmespath==1.0.1",
        "joblib==1.2.0",
        "json5==0.9.24",
        "jsonmerge==1.9.0",
        "jsonschema==4.17.3",
        "kombu==5.2.4",
        "langchain==0.1.11",
        "langchain-community==0.0.25",
        "llama_cpp_python==0.2.15",
        "lxml==4.9.2",
        "moviepy==1.0.3",
        "newspaper3k==0.2.8",
        "nltk==3.8.1",
        "numpy==1.24.3",
        "openai==1.13.3",
        "openpyxl==3.1.2",
        "pandas==2.1.4",
        "pdfminer==20191125",
        "pillow==10.3.0",
        "prometheus-client==0.17.1",
        "psutil==5.9.8",
        "pybase64==1.3.1",
        "pydantic==2.6.4",
        "pydub==0.25.1",
        "pyjwt==2.4.0",
        "pylint==2.17.4",
        "pymupdf==1.24.0",
        "pyshacl==0.25.0",
        "pytesseract==0.3.10",
        "pytextract==2.0.1",
        "python-dotenv==1.0.0",
        "python-docx==1.1.0",
        "python-pptx==0.6.23",
        "rdflib==7.0.0",
        "redis==5.0.3",
        "regex==2023.5.5",
        "requests==2.31.0",
        "requests_html==0.10.0",
        "slack-sdk==3.26.1",
        "slack-sdk==3.26.1",
        "spacy==3.7.2",
        "speechrecognition==3.10.1",
        "tika==2.6.0",
        "transformers==4.36.0",
        "unidecode==1.3.7",
        "sentence-transformers==2.2.2",
    ]))

    return exe

def make_embedded_resources(exe):
    return exe.to_embedded_resources()

def make_install(exe):
    # Create an object that represents our installed application file layout.
    files = FileManifest()

    # Add the generated executable to our install layout in the root directory.
    files.add_python_resource(".", exe)

    return files

# Tell PyOxidizer about the build targets defined above.
register_target("exe", make_exe)
register_target("resources", make_embedded_resources, depends=["exe"], default_build_script=True)
register_target("install", make_install, depends=["exe"], default=True)

# Resolve whatever targets the invoker of this configuration file is requesting
# be resolved.
resolve_targets()