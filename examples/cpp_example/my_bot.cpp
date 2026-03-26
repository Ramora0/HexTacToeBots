/*
 * Example C++ bot for HexTacToeBots.
 *
 * Quick start:
 *   1. cp -r examples/cpp_example bots/my_bot
 *   2. cd bots/my_bot && python setup.py build_ext --inplace
 *   3. python evaluate.py my_bot random_bot
 *
 * Your class needs:
 *   - get_move(game) -> list of (q, r) tuples
 *
 * Optional (if your bot manages its own time internally):
 *   - time_limit property (read/write) — framework syncs before each call
 */

#include <pybind11/pybind11.h>
#include <pybind11/stl.h>
#include <vector>

namespace py = pybind11;

class MyCppBot {
public:
    double time_limit;

    MyCppBot(double tl = 0.05) : time_limit(tl) {}

    py::list get_move(py::object game) {
        int moves_left = game.attr("moves_left_in_turn").cast<int>();

        py::list result;
        // TODO: replace with your own logic
        for (int i = 0; i < moves_left; i++) {
            result.append(py::make_tuple(0, 0));
        }
        return result;
    }
};

PYBIND11_MODULE(my_bot_cpp, m) {
    py::class_<MyCppBot>(m, "MyCppBot")
        .def(py::init<double>(), py::arg("time_limit") = 0.05)
        .def("get_move", &MyCppBot::get_move, py::arg("game"))
        .def_readwrite("time_limit", &MyCppBot::time_limit);
}
