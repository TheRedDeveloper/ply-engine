//% IMPORTS
import android.net.Uri;
import java.io.FileInputStream;
import java.io.InputStream;
import java.io.OutputStream;
//% END

//% MAIN_ACTIVITY_BODY
private static final int EXPORT_FILE_REQUEST_CODE = 0x706C79;
private String pendingExportSourcePath = null;

public void exportFile(String sourcePath, String fileName, String mimeType) {
	final String finalSourcePath = sourcePath;
	final String finalFileName = fileName;
	final String finalMimeType = (mimeType == null || mimeType.isEmpty())
		? "application/octet-stream"
		: mimeType;

	runOnUiThread(new Runnable() {
		@Override
		public void run() {
			try {
				pendingExportSourcePath = finalSourcePath;

				Intent intent = new Intent(Intent.ACTION_CREATE_DOCUMENT);
				intent.addCategory(Intent.CATEGORY_OPENABLE);
				intent.setType(finalMimeType);
				intent.putExtra(Intent.EXTRA_TITLE, finalFileName);

				startActivityForResult(intent, EXPORT_FILE_REQUEST_CODE);
			} catch (Exception e) {
				Log.e("SAPP", "exportFile: failed to open picker", e);
				pendingExportSourcePath = null;
			}
		}
	});
}
//% END

//% MAIN_ACTIVITY_ON_ACTIVITY_RESULT
if (requestCode == EXPORT_FILE_REQUEST_CODE) {
	try {
		if (
			resultCode == Activity.RESULT_OK &&
			data != null &&
			data.getData() != null &&
			pendingExportSourcePath != null
		) {
			Uri targetUri = data.getData();

			try (
				InputStream input = new FileInputStream(pendingExportSourcePath);
				OutputStream output = getContentResolver().openOutputStream(targetUri, "w")
			) {
				if (output != null) {
					byte[] buffer = new byte[8192];
					int read;
					while ((read = input.read(buffer)) != -1) {
						output.write(buffer, 0, read);
					}
					output.flush();
				} else {
					Log.e("SAPP", "exportFile: output stream is null");
				}
			}
		}
	} catch (Exception e) {
		Log.e("SAPP", "exportFile: failed to copy file", e);
	} finally {
		pendingExportSourcePath = null;
	}
	return;
}
//% END
