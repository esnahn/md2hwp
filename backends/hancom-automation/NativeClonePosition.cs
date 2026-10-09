namespace Md2Hwp.HancomIrPreview;

internal readonly record struct NativeCloneCursor(int List, int Paragraph, int Offset);

// Native InsertFile leaves the cursor at the new root anchor. The old empty
// terminal paragraph becomes that anchor and a new empty terminal follows it.
// Final document verification still proves content/format preservation.
internal static class NativeClonePosition
{
    internal static NativeCloneCursor Begin(object automation) => RequireEmptyEnd(Read(automation));

    internal static int Complete(object automation, NativeCloneCursor before)
    {
        RequireAnchor(before, Read(automation));
        dynamic hwp = automation;
        if (!(bool)hwp.HAction.Run("MoveDocEnd"))
            throw new InvalidOperationException("Could not reach the document end after native clone insertion.");
        return RequireEnd(before, Read(automation));
    }

    internal static NativeCloneCursor RequireEmptyEnd(NativeCloneCursor position)
    {
        if (position.List != 0 || position.Paragraph < 0 || position.Paragraph == int.MaxValue || position.Offset != 0)
            throw new InvalidOperationException("Native clone insertion requires an empty final root paragraph; no paragraph was added automatically.");
        return position;
    }

    internal static void RequireAnchor(NativeCloneCursor before, NativeCloneCursor after)
    {
        RequireEmptyEnd(before);
        if (after != before)
            throw new InvalidOperationException("Native clone insertion left an unexpected cursor; expected the new root anchor at the original document end.");
    }

    internal static int RequireEnd(NativeCloneCursor before, NativeCloneCursor end)
    {
        RequireEmptyEnd(before);
        if (end != new NativeCloneCursor(0, before.Paragraph + 1, 0))
            throw new InvalidOperationException("Native clone insertion must add exactly one root paragraph followed by an empty terminal paragraph.");
        return before.Paragraph;
    }

    private static NativeCloneCursor Read(object automation)
    {
        dynamic hwp = automation;
        int list, paragraph, offset;
        hwp.GetPos(out list, out paragraph, out offset);
        return new(list, paragraph, offset);
    }
}
