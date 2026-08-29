cmake_minimum_required(VERSION 3.22.1)
project(ping LANGUAGES C)

add_library(ping SHARED ping.c)
target_link_libraries(ping log)
