project('com.github.eppixx.buoy', 'rust',
          version: '1.0.2',
    meson_version: '>= 0.59.0',
  default_options: [ 'warning_level=2', 'werror=false', ],
)

exec_name = 'buoy'

i18n = import('i18n')
gnome = import('gnome')

pkgdatadir = join_paths(get_option('prefix'), get_option('datadir'), meson.project_name())

conf = configuration_data()
conf.set_quoted('APP_NAME', 'Buoy')
conf.set_quoted('APP_ID', meson.project_name())
conf.set('APP_ID_UNQUOTED', meson.project_name())
conf.set_quoted('SLASHED_APP_ID', '/' + meson.project_name().replace('.', '/') + '/')
conf.set('EXEC_NAME', exec_name)
conf.set_quoted('EXEC_NAME_QUOTED', exec_name)
conf.set_quoted('GETTEXT_PACKAGE', meson.project_name())
conf.set_quoted('LOCALEDIR', join_paths(get_option('prefix'), get_option('localedir')))
conf.set_quoted('PKGDATADIR', pkgdatadir)

subdir('data')
subdir('src')
subdir('po')

gnome.post_install(
     glib_compile_schemas: false,
    gtk_update_icon_cache: true,
  update_desktop_database: true,
)
