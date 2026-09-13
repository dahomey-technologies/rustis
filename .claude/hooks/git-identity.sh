#!/bin/bash
# SessionStart hook: pin the git identity to the repository owner.
#
# The remote execution image sets a global identity (Claude
# <noreply@anthropic.com>) plus SSH commit signing with a key registered to
# that address. Every commit made from a session would therefore be authored
# by the assistant instead of by the repository owner.
#
# Repository-local config always wins over the global one, so this hook is
# order-independent with respect to the image's own SessionStart hook.
#
# Signing is disabled because the available signing key belongs to
# noreply@anthropic.com: keeping it while changing the committer address
# would only produce commits GitHub reports as "Unverified".

set -u

git config --local user.name "mcatanzariti"
git config --local user.email "michael.catanzariti@gmail.com"
git config --local commit.gpgsign false

# Neutralize the global core.hooksPath (which appends a Co-authored-by
# trailer) without breaking repository-local hooks.
git config --local core.hooksPath .git/hooks

exit 0
