# Python-based tests

The directory contains Python-based tests.  The tests are run as part
of nightly testing locally.

There is no set format of what the tests do but they typical start
a local test cluster using unc-node binary at `../target/debug/unc-node`.
There is also some capacity of starting the cluster on remote
machines.

## Running tests

### Running tests locally

To run tests locally first compile a debug build of the infra
package, make sure that all required Python packages are installed and
then execute the test file using python.  For example:

    cargo build
    cd pytest
    python3 -m venv myenv
    source myenv/bin/activate
    python3 -m pip install -U -r requirements.txt
    python3 tests/loadtest/loadtest.py

After the test finishes, log files and other result data from running
each node will be located in a `~/.unc/test#_finished` directory
(where `#` is index of the node starting with zero).

Note that running the tests using `pytest` command is not supported
and won’t work reliably.

Furthermore, running multiple tests at once is not supported either
because tests often use hard-coded paths (e.g. `~/.node/test#` for
node home directories) and port numbers

## Creating new tests

Even though this directory is called `pytest`, the tests need to work
when executed via `python3`.  This means that they need to execute the
tests when run as the main module rather than just defining the tests
function.  To make that happen it’s best to implement the tests using
the python's unittest unc-infra.but trigger them manually from within
the `__main__` condition like so:

    if __name__ == "__main__":
        unittest.main()

Alternatively, using the legacy way, the tests can be defined as
`test_<foo>` functions with test bodies and than executed in
a code fragment guarded by `if __name__ == '__main__'` condition.

If the test operates on the nodes running in a cluster, it will very
likely want to make use of `start_cluster` function defined in the
`lib/cluster.py` module.

Rather than assuming location a temporary directory, well-behaved test
should use `tempfile` module instead which will automatically take
`TEMPDIR` variable into consideration.  This is especially important
for Pytest which will automatically cleanup after a test which
respects `TEMPDIR` directory even if that tests ends up not cleaning
up its temporary files.

### Code Style

To automate formatting and avoid excessive bike shedding, we're using
YAPF to format Python source code in the pytest directory.  It can be
installed from Python Package Index (PyPI) using `pip` tool:

    python3 -m pip install yapf

Once installed, it can be run either on a single file, for example
with the following command:

    python3 -m yapf -pi lib/cluster.py

or the entire directory with command as seen below:

    python3 -m yapf -pir .

The `-p` switch enables parallelism and `-i` applies the changes in
place.  Without the latter switch the tool will write formatted file
to standard output instead.

The command should be executed in the `pytest` directory so that it’ll
pick up configuration from the `.style.yapf` file.

### Productivity tips

- The pytest often rely on utilities
located in pytest/lib and are imported using the following statement:
`sys.path.append(str(pathlib.Path(__file__).resolve().parents[2] / 'lib'))`
In order to make VSCode see that import you can add this path to the python
extra paths config. In order to do so add the following into the settings file:

```json
    "python.analysis.extraPaths": [
        "pytest/lib"
    ]
```
