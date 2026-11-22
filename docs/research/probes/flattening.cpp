#include <algorithm>
#include <array>
#include <cmath>
#include <iomanip>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>
#if defined(GC)
#include "geometrycentral/surface/boundary_first_flattening.h"
#include "geometrycentral/surface/surface_mesh_factories.h"
#elif defined(IGL)
#include <igl/lscm.h>
#elif defined(PROBE_OPENABF)
#include <OpenABF/OpenABF.hpp>
#endif
using P = std::array<double, 3>;
using U = std::array<double, 2>;
using T = std::vector<size_t>;
struct Fixture {
    std::string name;
    std::vector<P> xyz;
    std::vector<T> faces;
};
double len(P a, P b) { return std::hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]); }
std::vector<U> solve(const Fixture &f) {
    std::vector<U> uv(f.xyz.size());
#if defined(GC)
    using namespace geometrycentral;
    using namespace geometrycentral::surface;
    std::vector<Vector3> positions;
    for (auto p : f.xyz)
        positions.push_back({p[0], p[1], p[2]});
    auto [mesh, geo] = makeManifoldSurfaceMeshAndGeometry(f.faces, positions);
    auto result = parameterizeBFFfromScaleFactors(*mesh, *geo, VertexData<double>(*mesh, 0.));
    for (auto v : mesh->vertices())
        uv[v.getIndex()] = {result[v].x, result[v].y};
#elif defined(IGL)
    Eigen::MatrixXd xyz(f.xyz.size(), 3), out;
    Eigen::MatrixXi tri(f.faces.size(), 3);
    for (size_t i = 0; i < f.xyz.size(); ++i)
        for (int j = 0; j < 3; ++j)
            xyz(i, j) = f.xyz[i][j];
    for (size_t i = 0; i < f.faces.size(); ++i)
        for (int j = 0; j < 3; ++j)
            tri(i, j) = int(f.faces[i][j]);
    Eigen::VectorXi pins(2);
    pins << 0, 1;
    Eigen::MatrixXd fixed(2, 2);
    fixed << 0, 0, len(f.xyz[0], f.xyz[1]), 0;
    if (!igl::lscm(xyz, tri, pins, fixed, out))
        throw std::runtime_error("lscm failed");
    for (size_t i = 0; i < f.xyz.size(); ++i)
        uv[i] = {out(i, 0), out(i, 1)};
#elif defined(PROBE_OPENABF)
    using Angles = OpenABF::ABFPlusPlus<double>;
    using Map = OpenABF::AngleBasedLSCM<double, Angles::Mesh>;
    auto mesh = Angles::Mesh::New();
    for (auto p : f.xyz)
        mesh->insert_vertex(p[0], p[1], p[2]);
    mesh->insert_faces(f.faces);
    std::size_t iters = 0;
    double gradient = 0;
    Angles::Compute(mesh, iters, gradient);
    std::cout << "iter=" << iters << " residual=" << gradient << " ";
    Map::Compute(mesh);
    for (const auto &v : mesh->vertices())
        uv[v->idx] = {v->pos[0], v->pos[1]};
#endif
    return uv;
}
void check(const Fixture &f) {
    auto uv = solve(f);
    for (auto u : uv)
        for (double c : u)
            if (!std::isfinite(c))
                throw std::runtime_error("invalid coordinate");
    auto d2 = [&](size_t a, size_t b) {
        return std::hypot(uv[a][0] - uv[b][0], uv[a][1] - uv[b][1]);
    };
    const double factor = len(f.xyz[0], f.xyz[1]) / d2(0, 1);
    if (!std::isfinite(factor) || factor <= 0)
        throw std::runtime_error("invalid scale");
    double err = 0, raw = 0, area = 0;
    unsigned negative = 0, positive = 0, zero = 0;
    for (auto t : f.faces) {
        const size_t a = t[0], b = t[1], c = t[2];
        for (auto [i, j] : std::vector<std::array<size_t, 2>>{{a, b}, {b, c}, {c, a}}) {
            const double length = len(f.xyz[i], f.xyz[j]);
            raw = std::max(raw, std::abs(d2(i, j) - length));
            err = std::max(err, std::abs(factor * d2(i, j) - length));
        }
        const auto x = f.xyz[a], y = f.xyz[b], z = f.xyz[c];
        const P p = {y[0] - x[0], y[1] - x[1], y[2] - x[2]},
                q = {z[0] - x[0], z[1] - x[1], z[2] - x[2]};
        const double twice = std::hypot(p[1] * q[2] - p[2] * q[1], p[2] * q[0] - p[0] * q[2],
                                        p[0] * q[1] - p[1] * q[0]);
        const double det = (uv[b][0] - uv[a][0]) * (uv[c][1] - uv[a][1]) -
                           (uv[b][1] - uv[a][1]) * (uv[c][0] - uv[a][0]);
        area = std::max(area, std::abs(std::abs(det) * factor * factor / twice - 1.));
        if (det < 0)
            ++negative;
        else if (det > 0)
            ++positive;
        else
            ++zero;
    }
    double seam = 0;
    if (f.name == "cylinder")
        seam = std::abs(d2(0, 1) - d2(16, 17)) * factor;
    if (f.name == "cone")
        seam = std::abs(d2(0, 1) - d2(0, 9)) * factor;
    std::cout << f.name << " factor=" << factor << " raw_edge_diff=" << raw
              << " max_edge_mm=" << err << " max_area_fraction=" << area
              << " raw_signed(+,-,0)=" << positive << "," << negative << "," << zero
              << " seam_mm=" << seam << "\n";
}
int main() {
    std::cout << std::setprecision(9);
    Fixture plane{"plane",
                  {{0, 0, 0}, {60, 0, 0}, {60, 40, 0}, {0, 40, 0}, {30, 20, 0}},
                  {{0, 1, 4}, {1, 2, 4}, {2, 3, 4}, {3, 0, 4}}};
    Fixture cylinder{"cylinder", {}, {}}, cone{"cone", {{0, 0, 80}}, {}};
    for (size_t i = 0; i <= 8; ++i) {
        const double theta = 2. * std::acos(-1.) * double(i % 8) / 8.;
        const P p = {25 * std::cos(theta), 25 * std::sin(theta), 0};
        cylinder.xyz.push_back(p);
        cylinder.xyz.push_back({p[0], p[1], 60});
        cone.xyz.push_back(p);
        if (i < 8) {
            cylinder.faces.push_back({2 * i, 2 * i + 2, 2 * i + 3});
            cylinder.faces.push_back({2 * i, 2 * i + 3, 2 * i + 1});
            cone.faces.push_back({0, i + 1, i + 2});
        }
    }
    Fixture cap{"hemisphere", {{0, 0, 25}}, {}};
    for (size_t i = 0; i < 8; ++i) {
        const double theta = 2. * std::acos(-1.) * double(i) / 8.;
        cap.xyz.push_back({25 * std::cos(theta), 25 * std::sin(theta), 0});
        cap.faces.push_back({0, i + 1, (i + 1) % 8 + 1});
    }
    const double angle = std::acos((1. + std::cos(std::acos(-1.) / 4.)) / 2.);
    std::cout << "cap_angle_deficit_rad=" << 2. * std::acos(-1.) - 8. * angle << "\n";
    for (auto *f : {&plane, &cylinder, &cone, &cap}) {
        try {
            check(*f);
        } catch (const std::exception &e) {
            std::cout << f->name << " FAILED " << e.what() << "\n";
            return 1;
        }
    }
}
