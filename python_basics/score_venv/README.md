# Virtualenv Bazel Integration

This module provides a virtualenv target to aid python development


---

## Directory Structure

```bash
├── BUILD                    
├── py_pyenv.bzl            # Contains the 'score_virtualenv' macro
└── integration-test/       # Testing to make sure virtualenv works
    └── ...
```

---

## Usage

In the consuming Bazel project:

### 1. In your `MODULE.bazel` import the python basics

```python
bazel_dep(name = "score_python_basics", version = "0.1.0")
```

### 2. In the `BUILD` file

```python
load("@score_python_basics//score_virtualenv:py_pyenv.bzl", "score_virtualenv")


# Use it with defaults
score_virtualenv()

# Add own requirements ontop
score_virtualenv(
    reqs = all_requiremnets # comming from your pip installation via '@pip...
) 

# Changing name (target name of the venv) or venv name is also possible
score_virtualenv(
    name = "docker_venv",
    venv_name = ".dev_docker_venv",
    reqs = all_requiremnets + dev_reqs 
)
```

You can also use the virtualenv as a target or dependency for other bazel targets or macros

---

## Output

Running the generated target like so: 

```bash
bazel run //:ide_support # this is the default target name
```
Will create the Python virtualenv. It will have:
* The provided requirements (at least everything from the 'score_python_basics')
* The name you provide in 'venv_name'. If none is provided it will be '.venv'
    

---

## Benefits

✅ Makes development of Python code easier inside Bazel
✅ Allows for multiple virtualenvs depending on specific needs
✅ Easy integration into Bazel projects that use Python
