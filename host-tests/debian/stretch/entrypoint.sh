#!/bin/bash

rakn | tee /dev/tty | grep "keyutils:1.5.9-9"
rakn | grep "Release: 9"
