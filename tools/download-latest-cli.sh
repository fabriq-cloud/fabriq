#!/bin/bash

curl -s https://api.github.com/repos/fabriq-cloud/fabriq/releases/latest | grep "fabriq" | cut -d : -f 2,3 | tr -d \" | wget -qi -
