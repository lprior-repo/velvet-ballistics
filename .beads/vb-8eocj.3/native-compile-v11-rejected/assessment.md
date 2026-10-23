# Actual native V11 compile failure, no test execution

The V11 Reduce v1 draft failed compilation with E0308 (expected unit, found bool) at runtime drive/actions.rs49. Exit101,9.286s, no timeout. Zero tests ran. This receipt does not apply to newer v2/v3 drafts.

Latest executed composite suite remains V10 on34fd304e7214ced5527d28768bb98aa240045b0b:12passed,7failed. Frozen19source fixtures unchanged; SHA25649bcff41f94eaed46dab343ea2b771e7b5aa70cb2151b2f91fc99792915d708e.

Repeated unsuccessful local drafts are a delivery risk, not implementation acceptance. Repeatv14 preserved921c7715cddebd981ba889fc054902e02c6f334d and Reducev2 preserved228394585cd57742e609f4ab97a3d0911211905e are rejected/uncompiled drafts. RepeatStart DOES have kind.body; the earlier coordination handoff saying otherwise was wrong. The current canonical lowerer sets outer next=None and kind.body=body_step. Reduce kind variants have no output field; it is outer node.output.

All F1/F2 acceptance remains open pending coherent repairs, actual native tests, independent CODE and TEST reviews, and canonical gates. No main integration acceptance.
