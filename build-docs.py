#!/usr/bin/env python
import subprocess
from distutils.dir_util import copy_tree
subprocess.call('cargo doc', shell=True)
copy_tree('target/doc', '../rz80-webpage')



