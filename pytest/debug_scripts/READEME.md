# Debug Scripts

## Content

* request_chain_info.py

  This script can be used to request blockchain info

* send_validator_logs.py

  This script can be used to send Validator logs to a S3 bucket when issues are encountered. The core team can use the logs to help the validators troubleshoot issues.

## Instruction to run test

Add debug_scripts to your PYTHONPATH

```sh
export PYTHONPATH="<absolute path>/debug_scripts:$PYTHONPATH"
```

```sh
cd <absolute path>/debug_scripts
python3 -m pip install pipenv
pipenv install
python3 -m pipenv shell
python3 -m pipenv sync

python3 send_validator_logs.py --help
OR
python3 request_chain_info.py --help

python3 -m unittest tests.send_validator_logs_test 
```
