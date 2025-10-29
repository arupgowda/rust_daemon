#!/bin/bash
count=5
while [ $count -gt 0 ]; do
  echo "$count: Home folder is: $HOME"
  let "count-=1"
  sleep 10
done
