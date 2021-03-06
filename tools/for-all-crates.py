#!/usr/bin/env python3

# This file is part of oraide.  See <https://github.com/Phrohdoh/oraide>.
# 
# oraide is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License version 3
# as published by the Free Software Foundation.
# 
# oraide is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
# 
# You should have received a copy of the GNU Affero General Public License
# along with oraide.  If not, see <https://www.gnu.org/licenses/>.

# This script allows us to run `cargo` commands for each crate in the project.
# It isn't very pretty, or even maintainable, but it works.
#
# Example invocations:
#
# $ ./tools/for-all-crates.py -q check
# $ ./tools/for-all-crates.py test -- --nocapture

import os
import sys
import pathlib
import subprocess

def get_subdirs_of(dir):
    return [ os.path.abspath(os.path.join(dir, name))
        for name in os.listdir(dir)
            if os.path.isdir(os.path.join(dir, name))
    ]


blacklisted_component_names = ['oraide-miniyaml']


def get_component_paths_in_dir(dir):
    component_dirs = []

    subdirs = get_subdirs_of(dir)
    for abs_path in subdirs:
        for blacklsited_comp_name in blacklisted_component_names:
            if abs_path.endswith(blacklsited_comp_name):
                continue
            else:
                component_dirs.append(abs_path)

    return component_dirs


known_cargo_commands = ['check', 'test', 'doc', 'build']
known_cargo_commands_display = ', '.join([f"'{name}'" for name in known_cargo_commands])

args = sys.argv[1:]
if not args:
    sys.exit(f"please provide one of {known_cargo_commands_display} as a command")

first_non_flag_arg = next((x for x in args if not x.startswith('-')), None)
if not first_non_flag_arg:
    sys.exit(f"please provide one of {known_cargo_commands_display} as a command")

if first_non_flag_arg == 'run':
    sys.exit("refusing to issue the 'run' command")

this_script_dir = sys.path[0]
proj_root_dir = os.path.abspath(os.path.join(this_script_dir, '..'))

run_proc = []

if first_non_flag_arg in known_cargo_commands:
    run_proc = ['cargo'] + args
else:
    sys.exit(f"please provide one of {known_cargo_commands_display} as a command")

if run_proc:
    run_proc_display = f"'{' '.join(run_proc)}'"

    print(f"Executing {run_proc_display} in {proj_root_dir}")
    subprocess.run(run_proc, cwd=proj_root_dir, check=True)

    for abs_path in get_component_paths_in_dir(os.path.join(proj_root_dir, 'components')):
        print(f"Executing {run_proc_display} in {abs_path}")
        subprocess.run(run_proc, cwd=abs_path, check=True)

else:
    sys.exit("no command to run")