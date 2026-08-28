# Two Audio Units in one containing app

**Symptom.** A containing app ships two app extensions, each declaring one `AudioComponents` entry. Both register —
`pluginkit -mAvvv` lists both — but `auval -a` publishes only one of them, and registering the second *removes* the
first. On the machine this was measured on it also removed unrelated Musa components registered from a different
containing app, so the failure reads as a corrupted registry rather than as a rejection.

**Measured, 2026-08-28**, macOS 26.0 (25G83), Xcode 26.0:

| Shape | `auval -a` |
| --- | --- |
| One appex, one component | publishes |
| Two appexes, one component each | publishes exactly one — the second registered wins regardless of order |
| One appex, two `AudioComponents` entries | publishes both |

Re-signing, restarting `AudioComponentRegistrar`, unregistering everything first, waiting sixty seconds, giving the two
appexes distinct `AudioComponentBundle` values, and changing the second component's subtype so no other component
claimed it — none of these changed the two-appex result.

**What to do.** Declare both components in one extension's `AudioComponents` array and dispatch in
`AUAudioUnitFactory.createAudioUnit(with:)` on `componentDescription.componentType`. That is what `apps/musa-audio-unit`
does: one `MusaComponents.appex` publishes `aumu musa Musa` and `aumi musp Musa`, and the factory returns a
`MusaInstrumentAudioUnit` or a `MusaProcessorAudioUnit`. The two audio units still share no mutable state — one bundle
is not one component.

**A second trap on top of it.** A stale registration of the *same* four-character triple from another build shadows a
new one silently. `apps/musa-audio-unit-trial` registers `aumi musp Musa`, the same triple prompt 215 validated and
prompt 218 ships, so a leftover trial registration makes the production processor look absent. Unregister it:

```sh
pluginkit -r <trial-app>/Contents/PlugIns/Processor.appex
```

`scripts/check-audio-unit.sh` registers and unregisters what it needs and leaves nothing behind; a leftover comes from
an interrupted run.

