#!/usr/bin/env bash
#
# Wait for the host package manager to go idle before we run one ourselves.
#
# GitHub's Ubuntu runners boot with `apt-daily` / `unattended-upgrades` timers
# that hold /var/lib/apt/lists/lock for the first few minutes. Playwright's
# `--with-deps` install shells out to apt under sudo, so a job that starts
# early races them. Both outcomes have bitten this repo: a hard failure
# ("Could not get lock") and an indefinite block that stalled two release runs
# past 20 minutes with no output.
#
# Stops the timers, then waits for the locks to clear. Best-effort throughout:
# a non-Linux host, a missing systemd unit, or a still-held lock all exit 0 and
# let the install proceed, because this is contention avoidance and not a gate.

set -uo pipefail

[ "$(uname -s)" = "Linux" ] || { echo "quiesce-apt: not Linux, nothing to do"; exit 0; }

# --no-block, because `systemctl stop unattended-upgrades` waits for the
# in-flight upgrade to finish and Ubuntu ships TimeoutStopSec=900 for it. A
# blocking stop would hang this step for up to 15 minutes and simply relocate
# the stall we are here to prevent. The bounded wait below is what does the
# actual work; asking the units to stop just helps it converge sooner.
for unit in apt-daily.timer apt-daily-upgrade.timer \
  apt-daily.service apt-daily-upgrade.service unattended-upgrades.service; do
  sudo systemctl stop --no-block "$unit" >/dev/null 2>&1 || true
done

LOCKS="/var/lib/apt/lists/lock /var/lib/dpkg/lock-frontend /var/lib/dpkg/lock"
DEADLINE=$((SECONDS + 180))
while [ "$SECONDS" -lt "$DEADLINE" ]; do
  # shellcheck disable=SC2086 # word splitting is intended for the lock list
  if ! sudo fuser $LOCKS >/dev/null 2>&1; then
    echo "quiesce-apt: package manager idle after ${SECONDS}s"
    exit 0
  fi
  sleep 2
done

echo "quiesce-apt: locks still held after ${SECONDS}s; continuing anyway"
exit 0
