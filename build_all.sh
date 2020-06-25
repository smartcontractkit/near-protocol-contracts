#!/bin/bash
cd near-link-token && ./build.sh && cd ..
cd oracle && ./build.sh && cd ..
cd client && ./build.sh && cd ..