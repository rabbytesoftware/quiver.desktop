// This target exists only to give `actool` an asset catalog to compile.
//
// Icon Composer `.icon` bundles are compiled by Xcode's asset catalog tool into
// an Assets.car, and actool only runs as part of building a real target — there
// is no supported standalone invocation for `folder.iconcomposer.icon` input.
// So the project below declares the smallest possible framework target, this
// file is its only source, and the .icon rides along as a resource. The product
// framework is discarded; only Resources/Assets.car and the generated .icns are
// copied out (see ../compile-icon.sh).
import Foundation

public class IconCompilerStub {}
