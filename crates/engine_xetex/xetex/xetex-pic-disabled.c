/* xetex-pic-disabled.c: diagnostics for builds without image probing
   Licensed under the MIT License.
*/

#include "xetex-core.h"
#include "xetex-xetexd.h"

int
count_pdf_file_pages(void)
{
    return _tt_abort(
        "this build of the XeTeX engine does not support PDF image inputs"
    );
}

void
load_picture(bool is_pdf)
{
    _tt_abort(
        "this build of the XeTeX engine does not support %s image inputs",
        is_pdf ? "PDF" : "raster"
    );
}
