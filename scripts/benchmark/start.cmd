@echo off
@rem This batch file is the entry point for sandbox to start the benchmark.
powershell.exe -command "&{Set-ExecutionPolicy RemoteSigned -Force}"
powershell.exe -noprofile -file c:\workspace\bench.ps1
@pause
