#!/bin/bash
count=5
while [ $count -gt 0 ]; do
    echo "$count: Current date and time is: $(date)"
  let "count-=1"
  sleep 10
done
