public class OptClass {

    public int square(int a) {
        return a * a;
    }


    public void doThing() {
        int y = 0;
        if (1 > square(1)) {
            y = 1;
        } else {
            y = 2;
        }
        int x = square(y);
    }

}