#!/bin/bash

git config --replace-all core.filemode true
find . -name target -prune -o -type f -exec chmod 600 {} +
find . -name target -prune -o -type d -exec chmod 700 {} +
find . -name target -prune -o -name '*.sh' -type f -exec chmod 700 {} +
