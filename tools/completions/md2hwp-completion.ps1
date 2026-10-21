# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (C) 2026 Euisoon Ahn
# Load in the current session or dot-source this file from $PROFILE.
# This script never launches md2hwp, Pandoc, .NET or Hancom.

& {
    $complete = {
        param($wordToComplete, $commandAst, $cursorPosition)

        $subcommands = @('md2ir', 'ir2hwp', 'validate', 'setup-pandoc',
            'check-runtime', 'validate-template', 'init-template')
        $options = @{
            root = @('--output', '--template', '--worker', '--dotnet', '--verbose', '--version', '--help', '-h')
            convert = @('--output', '--template', '--worker', '--dotnet', '--verbose')
            version = @('--worker', '--dotnet')
            md2ir = @('--input', '--output', '--from', '--pandoc', '--force', '--help', '-h')
            ir2hwp = @('--ir', '--output', '--template', '--worker', '--dotnet', '--verbose', '--help', '-h')
            validate = @('--input', '--pandoc', '--require-meta', '--help', '-h')
            'setup-pandoc' = @('--help', '-h')
            'check-runtime' = @('--dotnet', '--help', '-h')
            'validate-template' = @('--template', '--worker', '--dotnet', '--help', '-h')
            'init-template' = @('--output', '--worker', '--dotnet', '--help', '-h')
        }
        $valueOptions = @('--input', '--ir', '--output', '--template', '--worker',
            '--dotnet', '--from', '--pandoc', '--require-meta')
        $metadata = @('title', 'subtitle', 'author', 'date', 'date-meta', 'publisher', 'md2hwp-heading1-start')

        # Use the current command's AST, ignoring text to the right of the cursor.
        # Read literal values only; never evaluate expressions from the command line.
        $completed = [Collections.Generic.List[string]]::new()
        foreach ($element in @($commandAst.CommandElements | Select-Object -Skip 1)) {
            if ($element.Extent.EndOffset -ge $cursorPosition) { break }
            if ($element -is [Management.Automation.Language.StringConstantExpressionAst]) {
                $completed.Add($element.Value)
            } else {
                $completed.Add($element.Extent.Text)
            }
        }
        $mode = 'root'
        if ($completed.Count -gt 0) {
            if ($subcommands -contains $completed[0]) {
                $mode = $completed[0]
                $completed.RemoveAt(0)
            } elseif ($completed[0].EndsWith('.md', [StringComparison]::OrdinalIgnoreCase)) {
                $mode = 'convert'
                $completed.RemoveAt(0)
            } elseif ($completed.Contains('--version')) {
                $mode = 'version'
            } elseif (!$completed[0].StartsWith('-')) {
                # Do not guess options for an unknown subcommand.
                [Management.Automation.CompletionCompleters]::CompleteFilename($wordToComplete)
                return
            }
        }

        $used = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
        $values = @{}
        $pending = $null
        $positional = 0
        foreach ($argument in $completed) {
            if ($null -ne $pending) {
                $values[$pending] = $argument
                $pending = $null
            } elseif ($argument.StartsWith('-')) {
                $null = $used.Add($argument)
                if ($valueOptions -contains $argument) { $pending = $argument }
            } else {
                $positional++
            }
        }
        if ($mode -eq 'md2ir') {
            if ($used.Contains('--input') -and $positional -gt 0) {
                $null = $used.Add('--output')
            } elseif ($positional -gt 0) {
                $null = $used.Add('--input')
                if ($positional -gt 1) { $null = $used.Add('--output') }
            }
        }
        if ($mode -eq 'validate' -and $positional -gt 0) { $null = $used.Add('--input') }
        if ($mode -eq 'convert' -and $positional -gt 0) { $null = $used.Add('--output') }
        if ($mode -eq 'validate-template' -and $positional -gt 0) { $null = $used.Add('--template') }
        if ($mode -eq 'init-template' -and $positional -gt 0) { $null = $used.Add('--output') }

        $candidates = @()
        $kind = 'ParameterName'
        if ($null -ne $pending) {
            $kind = 'ParameterValue'
            if ($pending -eq '--from') {
                $candidates = if ($used.Contains('--pandoc')) { @('commonmark') } else { @('commonmark', 'pandoc-json') }
            } elseif ($pending -eq '--require-meta') {
                $candidates = $metadata
            } else {
                [Management.Automation.CompletionCompleters]::CompleteFilename($wordToComplete)
                return
            }
        } elseif ($wordToComplete.StartsWith('-')) {
            $candidates = @($options[$mode] | Where-Object {
                (!$used.Contains($_) -or $_ -eq '--require-meta') -and
                # Help is accepted alone after a subcommand, before other arguments.
                ($_ -notin @('--help', '-h') -or $completed.Count -eq 0)
            })
            if ($mode -eq 'md2ir' -and $values['--from'] -eq 'pandoc-json') {
                $candidates = @($candidates | Where-Object { $_ -ne '--pandoc' })
            }
        } elseif ($mode -eq 'root' -and $completed.Count -eq 0) {
            $kind = 'ParameterValue'
            $candidates = $subcommands
        }

        $matchingCandidates = @($candidates | Where-Object {
            $_.StartsWith($wordToComplete, [StringComparison]::OrdinalIgnoreCase)
        })
        foreach ($candidate in $matchingCandidates) {
            [Management.Automation.CompletionResult]::new($candidate, $candidate, $kind, $candidate)
        }
        # Keep native quoting, Unicode and directory navigation for file arguments.
        if ($null -eq $pending -and !$wordToComplete.StartsWith('-')) {
            [Management.Automation.CompletionCompleters]::CompleteFilename($wordToComplete)
        }
    }
    Register-ArgumentCompleter -Native -CommandName md2hwp, md2hwp.exe -ScriptBlock $complete
}
