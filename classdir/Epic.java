import java.lang.Throwable;
import com.exopteron.Sys;

public class Epic {
	public int x;

	Epic(int balls) {
		this.x = balls;
	}

	public int getX() {
		return this.x;
	}

	public static int cool(int epic) {
		String x = "When the imposter is sus!";
		String[] array = new String[1];
		array[0] = x;
		Sys.println(array);
		// try {
		// 	if (epic > 10) {
		// 		throw new Throwable();
		// 	}
		// } catch (Throwable t) {
		// 	return 42;
		// }
		return -69;
	}

	public static void swag() throws Throwable {
		try {
			throw new Throwable();
		} catch (Throwable t) {

		}
	}
}
