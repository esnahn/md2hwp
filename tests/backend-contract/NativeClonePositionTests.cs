using Md2Hwp.HancomIrPreview;

internal static class NativeClonePositionTests
{
    internal static void Run()
    {
        foreach (var paragraph in new[] { 0, 27, 826, int.MaxValue - 1 })
        {
            var before = new NativeCloneCursor(0, paragraph, 0);
            NativeClonePosition.RequireAnchor(before, before);
            if (NativeClonePosition.RequireEnd(before, new(0, paragraph + 1, 0)) != paragraph)
                throw new Exception("Native clone position lost its original root index.");
        }
        foreach (var before in new[] { new NativeCloneCursor(1, 27, 0), new(0, 27, 10), new(0, -1, 0), new(0, int.MaxValue, 0) })
            Reject(() => NativeClonePosition.RequireEmptyEnd(before));
        var anchor = new NativeCloneCursor(0, 27, 0);
        // A cell cursor, an automatically advanced cursor or a moved offset
        // must never be mistaken for the observed native clone anchor.
        foreach (var after in new[] { new NativeCloneCursor(1, 27, 0), new(0, 28, 0), new(0, 27, 1), new(0, 26, 0) })
            Reject(() => NativeClonePosition.RequireAnchor(anchor, after));
        // Zero/multiple inserted roots and text remaining in the new terminal
        // fail instead of silently adding paragraphs or guessing a location.
        foreach (var end in new[] { new NativeCloneCursor(0, 27, 0), new(0, 29, 0), new(0, 28, 1), new(1, 28, 0) })
            Reject(() => NativeClonePosition.RequireEnd(anchor, end));
    }

    private static void Reject(Action action)
    {
        try { action(); }
        catch (InvalidOperationException) { return; }
        throw new Exception("Unsafe native clone coordinates were accepted.");
    }
}
