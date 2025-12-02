#!/bin/bash

echo "--- System Information ---"
uname -a
lsb_release -d 2>/dev/null || cat /etc/*release 2>/dev/null | head -n 1
hostname
whoami
echo ""

echo "--- CPU Information ---"
lscpu | grep "Model name"
lscpu | grep "Architecture"
lscpu | grep "CPU(s)"
echo ""

echo "--- Memory Information ---"
free -h
echo ""

echo "--- Disk Usage ---"
df -h
echo ""

echo "--- Network Interfaces ---"
ip a show

echo "--------------------------------------------------------------------------"
